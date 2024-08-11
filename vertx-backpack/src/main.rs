#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

mod api;
mod ipc;
mod network;

use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

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
use esp_hal::uart::Uart;
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

    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::max(system.clock_control).freeze();
    let io = gpio::Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let rng = Rng::new(peripherals.RNG);
    let timg0 = timg::TimerGroup::new(peripherals.TIMG0, &clocks, None);
    let timg1 = timg::TimerGroup::new(peripherals.TIMG1, &clocks, None);

    let timers = make_static!([OneShotTimer::new(timg0.timer0.into())]);
    esp_hal_embassy::init(&clocks, timers);

    #[cfg(feature = "chip-esp32")]
    let (tx, rx) = (io.pins.gpio1, io.pins.gpio3);
    #[cfg(feature = "chip-esp32c3")]
    let (tx, rx) = (io.pins.gpio21, io.pins.gpio20);
    #[cfg(feature = "chip-esp32s3")]
    let (tx, rx) = (io.pins.gpio43, io.pins.gpio44);
    let (tx, rx) = Uart::new_async_with_default_pins(peripherals.UART0, &clocks, tx, rx)
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

    let init_acked = make_static!(AtomicBool::new(false));
    spawner.must_spawn(ipc::rx(
        rx,
        init_acked,
        start_network,
        api_responses.sender(),
    ));

    loop {
        ipc_tx_channel.send(ToMain::Init).await;
        Timer::after_millis(500).await;
        if init_acked.load(Ordering::Relaxed) {
            break;
        }
    }
}
