#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

extern crate alloc;

mod api;
mod ipc;
mod network;

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::gpio;
use esp_hal::prelude::*;
use esp_hal::rng::Rng;
use esp_hal::timer::timg;
use esp_hal::uart::config::Config as UartConfig;
use esp_hal::uart::Uart;
use portable_atomic::{AtomicU8, Ordering};
use static_cell::StaticCell;

use self::api::Api;

#[main]
async fn main(spawner: Spawner) {
    esp_alloc::heap_allocator!(32 * 1024);

    esp_println::logger::init_logger(log::LevelFilter::Info);
    log::info!("Logger initialized");

    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    let io = gpio::Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let rng = Rng::new(peripherals.RNG);
    let timg0 = timg::TimerGroup::new(peripherals.TIMG0);
    let timg1 = timg::TimerGroup::new(peripherals.TIMG1);

    esp_hal_embassy::init(timg0.timer0);

    #[cfg(feature = "chip-esp32")]
    let (rx, tx) = (io.pins.gpio17, io.pins.gpio16);
    #[cfg(feature = "chip-esp32c3")]
    let (rx, tx) = (io.pins.gpio20, io.pins.gpio21);
    #[cfg(feature = "chip-esp32s3")]
    let (rx, tx) = (io.pins.gpio5, io.pins.gpio4);
    let config = UartConfig::default().baudrate(vertx_backpack_ipc::BAUDRATE);
    let mut uart = Uart::new_async_with_config(peripherals.UART1, config, rx, tx).unwrap();

    #[ram(rtc_fast, persistent)]
    static BOOT_MODE: AtomicU8 = AtomicU8::new(0);

    let ipc = ipc::init(&mut uart, BOOT_MODE.load(Ordering::Relaxed)).await;
    static IPC: StaticCell<ipc::Context> = StaticCell::new();
    let ipc = IPC.init(ipc);

    let network = vertx_network_esp::Hal::new(
        spawner,
        rng,
        timg1.timer0.into(),
        peripherals.RADIO_CLK,
        peripherals.WIFI,
    );
    static API: StaticCell<api::Api> = StaticCell::new();
    let api = API.init_with(|| Api::new(ipc));
    let start_network = network::get_start(spawner, rng, network, api, ipc);

    let (rx, tx) = uart.split();
    spawner.must_spawn(ipc::rx(rx, &BOOT_MODE, start_network, api, ipc));
    spawner.must_spawn(ipc::tx(tx, ipc));
}
