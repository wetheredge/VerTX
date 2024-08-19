#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

mod api;
mod ipc;
mod network;

use core::mem::MaybeUninit;

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::clock::ClockControl;
use esp_hal::gpio;
use esp_hal::peripherals::Peripherals;
use esp_hal::prelude::*;
use esp_hal::rng::Rng;
use esp_hal::system::SystemControl;
use esp_hal::timer::{timg, OneShotTimer, PeriodicTimer};
use esp_hal::uart::config::Config as UartConfig;
use esp_hal::uart::Uart;
use portable_atomic::{AtomicBool, AtomicU8, Ordering};
use static_cell::make_static;
use vertx_backpack_ipc::ToMain;

use self::api::Api;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

/// Initialize the heap
///
/// # Safety
///
/// Must be called exactly once, before any allocations
pub unsafe fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    // SAFETY:
    // - `init_heap` is required to be called exactly once, before any allocations
    // - `HEAP_SIZE` is > 0
    unsafe { ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE) };
}

#[main]
async fn main(spawner: Spawner) {
    // SAFETY: main() will only run once
    unsafe { init_heap() };

    esp_println::logger::init_logger(log::LevelFilter::Info);
    log::info!("Logger initialized");

    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::max(system.clock_control).freeze();
    let io = gpio::Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let rng = Rng::new(peripherals.RNG);
    let timg0 = timg::TimerGroup::new(peripherals.TIMG0, &clocks, None);
    let timg1 = timg::TimerGroup::new(peripherals.TIMG1, &clocks, None);

    let timers = make_static!([OneShotTimer::new(timg0.timer0.into())]);
    esp_hal_embassy::init(&clocks, timers);

    #[cfg(any(feature = "chip-esp32", feature = "chip-esp32s3"))]
    let (tx, rx) = (io.pins.gpio4, io.pins.gpio5);
    #[cfg(feature = "chip-esp32c3")]
    let (tx, rx) = (io.pins.gpio21, io.pins.gpio20);
    let config = UartConfig::default().baudrate(vertx_backpack_ipc::BAUDRATE);
    let (tx, rx) = Uart::new_async_with_config(peripherals.UART0, config, &clocks, tx, rx)
        .unwrap()
        .split();

    let ipc_tx_channel = make_static!(ipc::TxChannel::new());
    spawner.must_spawn(ipc::tx(tx, ipc_tx_channel.receiver()));

    let network = vertx_network_esp::Hal::new(
        spawner,
        clocks,
        rng,
        PeriodicTimer::new(timg1.timer0.into()),
        peripherals.RADIO_CLK,
        peripherals.WIFI,
    );
    let api_responses = make_static!(api::ResponseChannel::new());
    let api = make_static!(Api::new(ipc_tx_channel.sender(), api_responses.receiver()));
    let start_network = network::get_start(spawner, rng, network, api);

    #[ram(rtc_fast, persistent)]
    static BOOT_MODE: AtomicU8 = AtomicU8::new(0);
    static INIT_ACKED: AtomicBool = AtomicBool::new(false);

    spawner.must_spawn(ipc::rx(
        rx,
        &BOOT_MODE,
        &INIT_ACKED,
        start_network,
        api_responses.sender(),
    ));

    loop {
        let boot_mode = BOOT_MODE.load(Ordering::Relaxed);
        ipc_tx_channel.send(ToMain::Init { boot_mode }).await;

        Timer::after_millis(500).await;
        if INIT_ACKED.load(Ordering::Relaxed) {
            break;
        }
    }
}
