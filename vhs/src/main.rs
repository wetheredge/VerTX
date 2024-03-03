#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

mod config;
mod crsf;
mod flash;
mod leds;
mod mode;
mod ota;
mod server;
mod wifi;

use alloc::vec::Vec;
use core::mem::MaybeUninit;

use embassy_executor::{task, Spawner};
use embassy_time::{Duration, Ticker, Timer};
use esp_backtrace as _;
use esp_hal_smartled::SmartLedsAdapter;
use hal::clock::ClockControl;
use hal::embassy::executor::Executor;
use hal::peripherals::Peripherals;
use hal::prelude::*;
use hal::rmt::Rmt;
use hal::timer::TimerGroup;
use hal::xtensa_lx::timer::get_cycle_count;
use hal::{embassy, Rng, IO};
use log::LevelFilter;
use portable_atomic::{AtomicU32, Ordering};
use static_cell::make_static;

pub use crate::config::Config;
pub use crate::mode::Mode;

const LOG_LEVEL: LevelFilter = LevelFilter::Info;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

/// Initialize the heap
///
/// # Safety
///
/// Must be called exactly once
unsafe fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
}

#[entry]
fn entry() -> ! {
    // SAFETY: entry() will only run once
    unsafe { init_heap() };

    let executor = make_static!(Executor::new());
    executor.run(main)
}

fn main(spawner: Spawner, idle_cycles: &'static AtomicU32) {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    esp_println::logger::init_logger(LOG_LEVEL);
    log::info!("Logger initialized");

    embassy::init(&clocks, TimerGroup::new(peripherals.TIMG0, &clocks));

    spawner.must_spawn(status(idle_cycles));

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let rmt = Rmt::new(peripherals.RMT, 80_u32.MHz(), &clocks).unwrap();

    let mode = make_static!(mode::Channel::new());

    let api_responses = make_static!(server::ApiResponseChannel::new());

    // Leds init
    {
        let leds = SmartLedsAdapter::new(
            rmt.channel0,
            io.pins.gpio38,
            [0; leds::BUFFER_SIZE],
            &clocks,
        );
        spawner.must_spawn(leds::run(leds, mode.subscriber().unwrap()));
    }

    flash::unlock().unwrap();
    let mut partitions = flash::read_partition_table()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let config = make_static!(Config::load(&mut partitions));

    // WiFi init
    if config.wifi.enable {
        let rng = Rng::new(peripherals.RNG);
        let timer = TimerGroup::new(peripherals.TIMG1, &clocks).timer0;

        let stack = wifi::run(
            &spawner,
            config,
            &clocks,
            timer,
            rng,
            peripherals.WIFI,
            system.radio_clock_control,
            mode.publisher(),
        );

        server::run(&spawner, stack, mode.publisher(), api_responses.receiver());
    }

    // spawner.must_spawn(simulate_arming(mode));
}

#[task]
async fn simulate_arming(mode: &'static mode::Publisher<'static>) {
    loop {
        Timer::after_secs(1).await;
        mode.publish(Mode::Armed);
        Timer::after_secs(2).await;
        mode.publish(Mode::Ok);
    }
}

#[task]
async fn status(idle_cycles: &'static AtomicU32) {
    log::info!("Starting status()");

    let mut ticker = Ticker::every(Duration::from_secs(1));
    idle_cycles.store(0, Ordering::Release);
    let mut start = get_cycle_count();
    loop {
        ticker.next().await;
        let idle_cycles = idle_cycles.swap(0, Ordering::AcqRel) as f32;
        let end = get_cycle_count();

        let total = end.wrapping_sub(start) as f32;

        let idle_percent = 100.0 * idle_cycles / total;
        let accuracy = 100.0 * (total - 24e7) / 24e7;

        #[cfg(debug_assertions)]
        if accuracy < f32::EPSILON {
            log::info!("Idle = {idle_percent:.02}%; Accuracy = 0%");
        } else {
            log::info!("Idle = {idle_percent:.02}%; Accuracy = {accuracy:+.04}%");
        }

        start = end;
    }
}
