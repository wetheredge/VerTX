#![no_std]
#![feature(type_alias_impl_trait)]

extern crate alloc;
#[cfg(feature = "simulator")]
extern crate std;

mod api;
#[cfg(feature = "backpack")]
mod backpack;
mod config;
mod crsf;
mod display;
mod hal;
mod leds;
mod mode;
mod mutex;
mod network;
mod reset;

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
    network: network::Config,
    expert: Mutex<mutex::SingleCore, bool>,
}

pub fn main(spawner: Spawner, idle_cycles: &'static AtomicU32) {
    log::info!("Starting VerTX");

    let hal::Init {
        mut rng,
        boot_mode,
        led_driver,
        config_storage,
        mode_button,
        network,
    } = hal::init(spawner);

    let mode = make_static!(mode::Channel::new());
    let status_signal = make_static!(api::StatusSignal::new());

    let config_manager = make_static!(config::Manager::new(config_storage));
    let config = config_manager.config();

    spawner.must_spawn(change_mode(boot_mode, mode_button));
    spawner.must_spawn(status(idle_cycles, status_signal));
    spawner.must_spawn(reset::reset(config_manager));
    spawner.must_spawn(leds::run(config, led_driver, mode.subscriber().unwrap()));

    if boot_mode.configurator_enabled() {
        log::info!("Configurator enabled");
        mode.publish(crate::Mode::PreConfigurator);

        let is_home = boot_mode == BootMode::ConfiguratorHome;

        let api = make_static!(api::Api::new(config_manager, status_signal));
        match network::run(spawner, is_home, config, &mut rng, network, api) {
            Ok(ok) => ok,
            Err(network::Error::InvalidHomeConfig) => {
                reset::reboot_into(BootMode::ConfiguratorField);
            }
        }
    } else {
        log::info!("Configurator disabled");
        mode.publish(crate::Mode::Ok);
    };
}

#[task]
async fn change_mode(boot_mode: BootMode, mut button: hal::ModeButton) {
    use hal::traits::ModeButton as _;
    use vertx_network_hal::Hal as _;

    button.wait_for_pressed().await;

    let mode = if boot_mode.configurator_enabled() {
        BootMode::Standard
    } else if hal::Network::SUPPORTS_HOME {
        BootMode::ConfiguratorHome
    } else if hal::Network::SUPPORTS_FIELD {
        BootMode::ConfiguratorField
    } else {
        unreachable!()
    };

    reset::reboot_into(mode);
}

#[task]
async fn status(idle_cycles: &'static AtomicU32, status: &'static api::StatusSignal) {
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

        status.signal(api::response::Status {
            battery_voltage: 0,
            idle_time,
            timing_drift,
        });

        start = end;
    }
}
