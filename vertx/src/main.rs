#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

mod pins {
    include!(concat!(env!("OUT_DIR"), "/pins.rs"));

    #[allow(unused)]
    pub(crate) use {pins, Pins};
}

mod config;
mod configurator;
mod crsf;
mod flash;
mod leds;
mod mode;
mod mutex;

use alloc::vec::Vec;
use core::mem::MaybeUninit;

use embassy_executor::{task, Spawner};
use embassy_time::{Duration, Ticker};
use esp_backtrace as _;
use esp_hal::clock::ClockControl;
use esp_hal::embassy;
use esp_hal::embassy::executor::Executor;
use esp_hal::gpio::IO;
use esp_hal::peripherals::Peripherals;
use esp_hal::prelude::*;
use esp_hal::rmt::Rmt;
use esp_hal::rng::Rng;
use esp_hal::timer::TimerGroup;
use esp_hal::xtensa_lx::timer::get_cycle_count;
use esp_hal_smartled::SmartLedsAdapter;
use log::LevelFilter;
use portable_atomic::{AtomicU32, Ordering};
use static_cell::make_static;

pub use crate::config::Config;
pub use crate::mode::Mode;
use crate::pins::pins;

const LOG_LEVEL: LevelFilter = LevelFilter::Info;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

/// Initialize the heap
///
/// # Safety
///
/// Must be called exactly once, before any allocations
unsafe fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    // SAFETY:
    // - `init_heap` is required to be called exactly once, before any allocations
    // - `HEAP_SIZE` is > 0
    unsafe { ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE) };
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

    embassy::init(&clocks, TimerGroup::new_async(peripherals.TIMG0, &clocks));

    let status_signal = make_static!(configurator::server::StatusSignal::new());
    spawner.must_spawn(status(idle_cycles, status_signal));

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let rmt = Rmt::new(peripherals.RMT, 80u32.MHz(), &clocks, None).unwrap();

    let mode = make_static!(mode::Channel::new());

    // Leds init
    {
        let leds = SmartLedsAdapter::new(
            rmt.channel0,
            pins!(io.pins, leds),
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
    let config = make_static!(config::Manager::new(&mut partitions));

    let configurator_enabled = configurator::IsEnabled::new();
    spawner.must_spawn(configurator::toggle_button(
        pins!(io.pins, configurator).into_pull_up_input().into(),
        configurator_enabled,
    ));

    if configurator_enabled.is_enabled() {
        log::info!("Configurator enabled");
        mode.publish(crate::Mode::PreConfigurator);

        let rng = Rng::new(peripherals.RNG);
        let timer = TimerGroup::new(peripherals.TIMG1, &clocks, None).timer0;

        let stack = configurator::wifi::run(
            &spawner,
            config.boot_config(),
            &clocks,
            timer,
            rng,
            peripherals.WIFI,
            system.radio_clock_control,
        );

        configurator::server::run(&spawner, stack, mode.publisher(), status_signal);
    } else {
        log::info!("Configurator disabled");
        mode.publish(crate::Mode::Ok);
    }
}

#[task]
async fn status(
    idle_cycles: &'static AtomicU32,
    status: &'static configurator::server::StatusSignal,
) {
    log::info!("Starting status()");

    let mut ticker = Ticker::every(Duration::from_secs(1));
    idle_cycles.store(0, Ordering::Release);
    let mut start = get_cycle_count();
    loop {
        ticker.next().await;
        let idle_cycles = idle_cycles.swap(0, Ordering::AcqRel) as f32;
        let end = get_cycle_count();

        let total = end.wrapping_sub(start) as f32;

        let idle_time = idle_cycles / total;
        let timing_drift = (total - 24e7) / 24e7;

        if cfg!(debug_assertions) {
            let idle_time = idle_time * 100.0;
            let timing_drift = timing_drift * 100.0;
            if timing_drift < f32::EPSILON {
                log::info!("Idle = {idle_time:.02}%; Drift = 0%");
            } else {
                log::info!("Idle = {idle_time:.02}%; Drift = {timing_drift:+.04}%");
            }
        }

        status.signal(vertx_api::response::Status {
            battery_voltage: 0,
            idle_time,
            timing_drift,
        });

        start = end;
    }
}
