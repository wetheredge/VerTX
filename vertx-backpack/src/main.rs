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
use esp_hal::prelude::*;
use esp_hal::rng::Rng;
use esp_hal::timer::timg;
use esp_hal::uart::{self, Uart};
use static_cell::StaticCell;

use self::api::Api;

#[main]
async fn main(spawner: Spawner) {
    esp_alloc::heap_allocator!(32 * 1024);

    esp_println::logger::init_logger(loog::log::LevelFilter::Info);
    loog::info!("Logger initialized");

    let p = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    let rng = Rng::new(p.RNG);
    let timg0 = timg::TimerGroup::new(p.TIMG0);
    let timg1 = timg::TimerGroup::new(p.TIMG1);

    esp_hal_embassy::init(timg0.timer0);

    #[cfg(feature = "chip-esp32")]
    let (rx, tx) = (p.GPIO17, p.GPIO16);
    #[cfg(feature = "chip-esp32c3")]
    let (rx, tx) = (p.GPIO20, p.GPIO21);
    #[cfg(feature = "chip-esp32s3")]
    let (rx, tx) = (p.GPIO5, p.GPIO4);
    let config = uart::Config::default().baudrate(vertx_backpack_ipc::BAUDRATE);
    let mut uart = Uart::new_with_config(p.UART1, config, rx, tx)
        .unwrap()
        .into_async();

    let ipc = ipc::init(&mut uart).await;
    static IPC: StaticCell<ipc::Context> = StaticCell::new();
    let ipc = IPC.init(ipc);

    let network =
        vertx_network_esp::Hal::new(spawner, rng, timg1.timer0.into(), p.RADIO_CLK, p.WIFI);
    static API: StaticCell<api::Api> = StaticCell::new();
    let api = API.init_with(|| Api::new(ipc));
    let start_network = network::get_start(spawner, rng, network, api, ipc);

    let (rx, tx) = uart.split();
    spawner.must_spawn(ipc::rx(rx, start_network, api, ipc));
    spawner.must_spawn(ipc::tx(tx, ipc));
}
