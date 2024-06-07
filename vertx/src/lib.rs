#![no_std]
#![feature(type_alias_impl_trait)]

extern crate alloc;
#[cfg(feature = "simulator")]
extern crate std;

mod config;
mod configurator;
mod crsf;
mod display;
pub mod hal;
mod leds;
mod mode;
mod mutex;
mod reset;
mod wifi;

use embassy_executor::{task, Spawner};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Ticker};
use portable_atomic::{AtomicU32, Ordering};
use static_cell::make_static;

pub use crate::mode::Mode;
use crate::reset::BootMode;

#[derive(Default, vertx_config::Storage, vertx_config::UpdateRef)]
struct Config {
    name: Mutex<mutex::SingleCore, heapless::String<20>>,
    leds: leds::Config,
    display: display::Config,
    wifi: wifi::Config,
    expert: Mutex<mutex::SingleCore, bool>,
}

pub fn main(spawner: Spawner, idle_cycles: &'static AtomicU32) {
    // SAFETY: Nothing before this will trigger a reset
    let reset = unsafe { reset::Manager::new() };

    log::info!("Starting VerTX");

    let hal::Init {
        mut rng,
        led_driver,
        config_storage,
        get_mode_button,
        get_net_driver,
    } = hal::init(spawner);

    let mode = make_static!(mode::Channel::new());
    let status_signal = make_static!(configurator::StatusSignal::new());

    let config_manager = make_static!(config::Manager::new(config_storage));
    let config = config_manager.config();

    spawner.must_spawn(change_mode(get_mode_button, reset));
    spawner.must_spawn(status(idle_cycles, status_signal));
    spawner.must_spawn(reset::reset(config_manager));
    spawner.must_spawn(leds::run(config, led_driver, mode.subscriber().unwrap()));

    match reset.current_mode() {
        BootMode::Standard => {
            log::info!("Configurator disabled");
            mode.publish(crate::Mode::Ok);
        }
        BootMode::Configurator => {
            log::info!("Configurator enabled");
            mode.publish(crate::Mode::PreConfigurator);

            let stack = wifi::run(spawner, config, &mut rng, get_net_driver);

            configurator::run(
                spawner,
                reset,
                config_manager,
                stack,
                mode.publisher(),
                status_signal,
            );
        }
    }
}

#[task]
async fn change_mode(get_mode_button: crate::hal::GetModeButton, reset: crate::reset::Manager) {
    use hal::traits::ModeButton;

    let mut button = get_mode_button();
    button.wait_for_pressed().await;
    reset.toggle_configurator();
}

#[task]
async fn status(idle_cycles: &'static AtomicU32, status: &'static configurator::StatusSignal) {
    log::info!("Starting status()");

    let mut ticker = Ticker::every(Duration::from_secs(1));
    idle_cycles.store(0, Ordering::Release);
    let mut start = hal::get_cycle_count();
    loop {
        ticker.next().await;
        let idle_cycles = idle_cycles.swap(0, Ordering::AcqRel) as f32;
        let end = hal::get_cycle_count();

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

        status.signal(configurator::protocol::response::Status {
            battery_voltage: 0,
            idle_time,
            timing_drift,
        });

        start = end;
    }
}
