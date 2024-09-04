mod flash;

use alloc::vec;
use alloc::vec::Vec;
use core::future::Future;

use embassy_executor::Spawner;
use esp_hal::clock::ClockControl;
use esp_hal::gpio;
use esp_hal::peripherals::Peripherals;
use esp_hal::prelude::*;
use esp_hal::rmt::Rmt;
use esp_hal::rng::Rng;
use esp_hal::system::SystemControl;
use esp_hal::timer::{timg, OneShotTimer, PeriodicTimer};
use esp_hal_smartled::SmartLedsAdapter;
use portable_atomic::{AtomicU8, Ordering};
use static_cell::make_static;
use {esp_backtrace as _, esp_println as _};

use self::flash::Partition;
use crate::BootMode;

#[ram(rtc_fast, persistent)]
static BOOT_MODE: AtomicU8 = AtomicU8::new(0);

pub(crate) fn init(spawner: Spawner) -> super::Init {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::max(system.clock_control).freeze();
    let io = gpio::Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let rmt = Rmt::new(peripherals.RMT, 80u32.MHz(), &clocks, None).unwrap();
    let rng = Rng::new(peripherals.RNG);
    let timg0 = timg::TimerGroup::new(peripherals.TIMG0, &clocks, None);
    let timg1 = timg::TimerGroup::new(peripherals.TIMG1, &clocks, None);

    let timers = make_static!([OneShotTimer::new(timg0.timer0.into())]);
    esp_hal_embassy::init(&clocks, timers);

    let led_driver = SmartLedsAdapter::new(
        rmt.channel0,
        pins!(io, leds),
        [0; 3 * 8 + 1], // 3 channels * 8 bits + 1 stop byte
        &clocks,
    );

    flash::unlock().unwrap();
    let mut partitions = flash::read_partition_table()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let config_storage = ConfigStorage::new(&mut partitions);

    let mode_button = gpio::AnyInput::new(pins!(io, mode), gpio::Pull::Up);

    super::Init {
        reset: Reset,
        rng,
        boot_mode: BootMode::from(BOOT_MODE.load(Ordering::Relaxed)),
        led_driver,
        config_storage,
        mode_button,
        network: vertx_network_esp::Hal::new(
            spawner,
            clocks,
            rng,
            PeriodicTimer::new(timg1.timer0.into()),
            peripherals.RADIO_CLK,
            peripherals.WIFI,
        ),
    }
}

pub(crate) fn set_boot_mode(mode: u8) {
    BOOT_MODE.store(mode, Ordering::Relaxed);
}

struct Reset;

impl super::traits::Reset for Reset {
    fn shut_down(&mut self) -> ! {
        panic!("Emulating shut down")
    }

    fn reboot(&mut self) -> ! {
        esp_hal::reset::software_reset();
        unreachable!()
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
            let mut config = vec![0; length];
            self.partition.read_into(1, &mut config).unwrap();

            let bytes: &[u8] = &bytemuck::cast_slice(&config)[0..length];
            parse(bytes)
        })
    }

    fn save(&mut self, mut data: Vec<u8>) {
        // Round up to the nearest u32
        data.resize(((data.len() + 3) / 4) * 4, 0);

        let words = bytemuck::cast_slice(&data);

        let words_len = words.len() as u32;
        let sectors = (words_len + 1) / (flash::SECTOR_BYTES / 4);
        for i in 0..sectors {
            self.partition.erase_sector(i).unwrap();
        }

        self.partition.write(0, &[words_len]).unwrap();
        self.partition.write(1, words).unwrap();
    }
}

impl super::traits::ModeButton for gpio::AnyInput<'_> {
    fn wait_for_pressed(&mut self) -> impl Future<Output = ()> {
        self.wait_for_falling_edge()
    }
}
