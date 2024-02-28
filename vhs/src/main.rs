#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

mod config;
mod crsf;
mod flash;
mod leds;
mod ota;
mod server;
mod status;
mod wifi;

use alloc::vec::Vec;
use core::mem::MaybeUninit;

use embassy_executor::{task, Spawner};
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal_smartled::SmartLedsAdapter;
use hal::clock::ClockControl;
use hal::peripherals::Peripherals;
use hal::prelude::*;
use hal::rmt::Rmt;
use hal::timer::TimerGroup;
use hal::{embassy, Rng, IO};
use log::LevelFilter;
use static_cell::make_static;

pub use crate::config::Config;
pub use crate::status::Status;

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

#[main]
async fn main(spawner: Spawner) {
    // SAFETY: main() will only run once
    unsafe { init_heap() };

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    esp_println::logger::init_logger(LOG_LEVEL);
    log::info!("Logger initialized");

    embassy::init(&clocks, TimerGroup::new(peripherals.TIMG0, &clocks));

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let rmt = Rmt::new(peripherals.RMT, 80_u32.MHz(), &clocks).unwrap();

    let status = &*make_static!(status::Channel::new());

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

    flash::unlock().unwrap();
    let mut partitions = flash::read_partition_table()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let config = &*make_static!(Config::load(&mut partitions));

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
            status.publisher(),
        );

        server::run(
            &spawner,
            stack,
            status.publisher(),
            api_responses.receiver(),
        );
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
