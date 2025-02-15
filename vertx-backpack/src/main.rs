#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

extern crate alloc;

mod api;
mod ipc;
mod network;

use embassy_executor::Spawner;
use esp_hal::rng::Rng;
use esp_hal::timer::timg;
use esp_hal::uart::{self, Uart};
use static_cell::StaticCell;
use {defmt_rtt as _, esp_backtrace as _};

use self::api::Api;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_alloc::heap_allocator!(32 * 1024);

    loog::info!("Logger initialized");

    let p = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = esp_hal::clock::CpuClock::max();
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
    let config = uart::Config::default().with_baudrate(vertx_backpack_ipc::BAUDRATE);
    let mut uart = Uart::new(p.UART1, config)
        .unwrap()
        .with_rx(rx)
        .with_tx(tx)
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
