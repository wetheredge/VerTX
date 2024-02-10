#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

mod crsf;
mod leds;
mod ota;
mod server;
mod status;

use core::convert::identity;
use core::mem::MaybeUninit;

use embassy_executor::{task, Spawner};
use embassy_net::Stack;
use embassy_time::Timer;
use embedded_svc::wifi::Wifi;
use esp_backtrace as _;
use esp_hal_smartled::SmartLedsAdapter;
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiStaDevice};
use esp_wifi::{wifi, EspWifiInitFor};
use hal::clock::ClockControl;
use hal::peripherals::Peripherals;
use hal::prelude::*;
use hal::rmt::Rmt;
use hal::timer::TimerGroup;
use hal::{embassy, Rng, IO};
use log::LevelFilter;
use static_cell::make_static;

pub use crate::status::Status;

const WIFI_SSID: &str = env!("SSID");
const WIFI_PASSWORD: &str = env!("PASSWORD");
const HOSTNAME: &str = "vhs";

const LOG_LEVEL: LevelFilter = LevelFilter::Info;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

#[main]
async fn main(spawner: Spawner) {
    init_heap();
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    esp_println::logger::init_logger(LOG_LEVEL);
    log::info!("Logger initialized");

    embassy::init(&clocks, TimerGroup::new(peripherals.TIMG0, &clocks));

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let rmt = Rmt::new(peripherals.RMT, 80_u32.MHz(), &clocks).unwrap();

    let status = &*make_static!(status::Channel::new());
    let status_publisher = status.publisher();

    let api_responses = &*make_static!(server::ApiResponseChannel::new());

    // Leds init
    {
        let leds = SmartLedsAdapter::new(
            rmt.channel0,
            io.pins.gpio38,
            [0; leds::BUFFER_SIZE],
            &clocks,
        );
        spawner.must_spawn(leds::run(leds, status.subscriber().unwrap()));
    }

    // WiFi init
    {
        status_publisher.publish(Status::PreWiFi);

        let mut rng = Rng::new(peripherals.RNG);

        let timer = TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
        let init = esp_wifi::initialize(
            EspWifiInitFor::Wifi,
            timer,
            rng.clone(),
            system.radio_clock_control,
            &clocks,
        )
        .unwrap();

        let (wifi_interface, controller) =
            wifi::new_with_mode(&init, peripherals.WIFI, WifiStaDevice).unwrap();

        let mut dhcp_config = embassy_net::DhcpConfig::default();
        dhcp_config.hostname = Some(HOSTNAME.try_into().unwrap());

        let dhcp = embassy_net::Config::dhcpv4(dhcp_config);
        let stack = &*make_static!(embassy_net::Stack::new(
            wifi_interface,
            dhcp,
            make_static!(embassy_net::StackResources::<{ server::TASKS + 1 }>::new()),
            ((rng.random() as u64) << 32) | (rng.random() as u64),
        ));

        spawner.must_spawn(connection(controller));
        spawner.must_spawn(network(stack));

        server::run(
            &spawner,
            stack,
            status.publisher(),
            api_responses.receiver(),
        );

        stack.wait_config_up().await;
        Timer::after_secs(2).await;
    }

    // spawner.must_spawn(simulate_arming(status_signal));
}

#[task]
async fn simulate_arming(status: &'static status::Publisher<'static>) {
    loop {
        Timer::after_secs(1).await;
        status.publish(Status::Armed);
        Timer::after_secs(2).await;
        status.publish(Status::Ok);
    }
}

#[task]
async fn connection(mut controller: WifiController<'static>) -> ! {
    use embedded_svc::wifi;

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
                ssid: WIFI_SSID.try_into().unwrap(),
                bssid: None,
                auth_method: wifi::AuthMethod::WPA2Personal,
                password: WIFI_PASSWORD.try_into().unwrap(),
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

#[task]
async fn network(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) -> ! {
    stack.run().await
}
