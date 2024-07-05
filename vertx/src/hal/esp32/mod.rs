mod pins {
    include!(concat!(env!("OUT_DIR"), "/pins.rs"));

    #[allow(unused)]
    pub(crate) use {pins, Pins};
}

mod flash;

use alloc::vec;
use alloc::vec::Vec;
use core::convert::identity;
use core::future::Future;

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::clock::ClockControl;
use esp_hal::gpio;
use esp_hal::peripherals::Peripherals;
use esp_hal::prelude::*;
use esp_hal::rmt::Rmt;
use esp_hal::rng::Rng;
use esp_hal::system::SystemControl;
use esp_hal::timer::timg;
use esp_hal_smartled::SmartLedsAdapter;
use esp_wifi::wifi::{self, WifiController, WifiEvent, WifiStaDevice};
use esp_wifi::EspWifiInitFor;
use heapless::String;

use self::flash::Partition;
use self::pins::pins;
use crate::BootMode;

pub(crate) fn init(spawner: Spawner) -> super::Init {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::max(system.clock_control).freeze();
    let io = gpio::Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let rmt = Rmt::new(peripherals.RMT, 80u32.MHz(), &clocks, None).unwrap();
    let rng = Rng::new(peripherals.RNG);

    esp_hal_embassy::init(
        &clocks,
        timg::TimerGroup::new_async(peripherals.TIMG0, &clocks),
    );

    let led_driver = SmartLedsAdapter::new(
        rmt.channel0,
        pins!(io.pins, leds),
        [0; crate::leds::BUFFER_SIZE],
        &clocks,
    );

    flash::unlock().unwrap();
    let mut partitions = flash::read_partition_table()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let config_storage = ConfigStorage::new(&mut partitions);

    let get_mode_button = || gpio::AnyInput::new(pins!(io.pins, configurator), gpio::Pull::Up);

    let get_net_driver = move |ssid, password| {
        let timers = timg::TimerGroup::new(peripherals.TIMG1, &clocks, None);

        let init = esp_wifi::initialize(
            EspWifiInitFor::Wifi,
            timers.timer0,
            rng,
            peripherals.RADIO_CLK,
            &clocks,
        )
        .unwrap();

        let (device, controller): (wifi::WifiDevice<'static, WifiStaDevice>, _) =
            wifi::new_with_mode(&init, peripherals.WIFI, WifiStaDevice).unwrap();

        spawner.must_spawn(connection(controller, ssid, password));

        device
    };

    super::Init {
        rng,
        boot_mode: BootMode::default(),
        led_driver,
        config_storage,
        get_mode_button,
        get_net_driver,
    }
}

pub(crate) fn set_boot_mode(_mode: u8) {
    todo!()
}

pub(crate) fn shut_down() -> ! {
    panic!("Emulating shut down")
}

pub(crate) fn reboot() -> ! {
    esp_hal::reset::software_reset();
    unreachable!()
}

pub(crate) fn get_cycle_count() -> u32 {
    esp_hal::xtensa_lx::timer::get_cycle_count()
}

impl super::traits::Rng for Rng {
    fn u32(&mut self) -> u32 {
        self.random()
    }
}

struct ConfigStorage {
    partition: Partition,
}

impl ConfigStorage {
    fn new(partitions: &mut Vec<Partition>) -> Self {
        let partition = partitions.iter().position(Partition::is_config).unwrap();
        Self {
            partition: partitions.swap_remove(partition),
        }
    }
}

impl super::traits::ConfigStorage for ConfigStorage {
    fn load<T>(&self, parse: impl FnOnce(&[u8]) -> T) -> Option<T> {
        let mut length = [0; 1];
        self.partition.read_into(0, &mut length).unwrap();
        let [length] = length;
        let length = if length == u32::MAX { 0 } else { length };
        let length = length as usize;

        (length > 0).then(|| {
            let mut config = vec![0; length.div_ceil(4)];
            self.partition.read_into(1, &mut config).unwrap();

            let bytes: &[u8] = &bytemuck::cast_slice(&config)[..length];
            parse(bytes)
        })
    }

    fn save(&mut self, data: &[u32]) {
        self.partition.erase_and_write(0, data).unwrap();
    }
}

impl super::traits::ModeButton for gpio::AnyInput<'_> {
    fn wait_for_pressed(&mut self) -> impl Future<Output = ()> {
        self.wait_for_falling_edge()
    }
}

#[embassy_executor::task]
async fn connection(
    mut controller: WifiController<'static>,
    ssid: &'static String<32>,
    password: &'static String<64>,
) -> ! {
    log::info!("Starting connection()");

    loop {
        // If connected, wait for disconnect
        if controller.is_connected().is_ok_and(identity) {
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            log::info!("WiFi disconnected");
            Timer::after_secs(1).await;
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let config = wifi::Configuration::Client(wifi::ClientConfiguration {
                ssid: ssid.clone(),
                bssid: None,
                auth_method: wifi::AuthMethod::WPA2Personal,
                password: password.clone(),
                channel: None,
            });

            controller.set_configuration(&config).unwrap();
            log::info!("Starting WiFi");
            controller.start().await.unwrap();
            log::info!("WiFi started");
        }

        log::info!("Connecting...");
        match controller.connect().await {
            Ok(()) => log::info!("WiFi connected"),
            Err(err) => {
                log::info!("WiFi connection failed: {err:?}");
                Timer::after_secs(5).await;
            }
        }
    }
}
