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
use static_cell::make_static;

pub(crate) use crate::mode::Mode;
use crate::reset::BootMode;

#[derive(Default, vertx_config::Storage, vertx_config::UpdateRef)]
struct Config {
    name: Mutex<mutex::SingleCore, heapless::String<20>>,
    leds: leds::Config,
    display: display::Config,
    network: network::Config,
    expert: Mutex<mutex::SingleCore, bool>,
}

pub async fn main(spawner: Spawner) {
    loog::info!("Starting VerTX");

    let hal::Init {
        reset,
        mut rng,
        #[cfg(not(feature = "backpack-boot-mode"))]
        boot_mode,
        led_driver,
        config_storage,
        mode_button,
        #[cfg(feature = "backpack")]
        backpack,
        #[cfg(not(feature = "network-backpack"))]
        network,
    } = hal::init(spawner);

    #[cfg(feature = "backpack")]
    let backpack = make_static!(backpack::Backpack::new(spawner, backpack));
    #[cfg(feature = "backpack-boot-mode")]
    let boot_mode = backpack.boot_mode().await;

    let mode = make_static!(mode::Channel::new());

    let config_manager = make_static!(config::Manager::new(config_storage));
    let config = config_manager.config();

    let reset = make_static!(reset::Manager::new(
        spawner,
        reset,
        config_manager,
        #[cfg(feature = "backpack")]
        backpack
    ));

    spawner.must_spawn(change_mode(boot_mode, reset, mode_button));
    spawner.must_spawn(leds::run(config, led_driver, mode.subscriber().unwrap()));

    if boot_mode.configurator_enabled() {
        loog::info!("Configurator enabled");
        mode.publish(crate::Mode::PreConfigurator);

        let is_home = boot_mode == BootMode::ConfiguratorHome;

        let api = make_static!(api::Api::new(spawner, reset, config_manager));
        let network_result = network::run(
            spawner,
            is_home,
            config,
            &mut rng,
            api,
            #[cfg(feature = "network-native")]
            network,
            #[cfg(feature = "network-backpack")]
            backpack,
        )
        .await;
        match network_result {
            Ok(ok) => ok,
            Err(network::Error::InvalidHomeConfig) => {
                reset.reboot_into(BootMode::ConfiguratorField).await;
            }
        }
    } else {
        loog::info!("Configurator disabled");
        mode.publish(crate::Mode::Ok);
    }
}

#[task]
async fn change_mode(
    boot_mode: BootMode,
    reset: &'static reset::Manager,
    mut button: hal::ModeButton,
) {
    use hal::traits::ModeButton as _;

    button.wait_for_pressed().await;

    let mode = if boot_mode.configurator_enabled() {
        BootMode::Standard
    } else {
        #[allow(unused_variables)]
        let try_home = true;
        #[cfg(feature = "network-native")]
        let try_home = <hal::Network as vertx_network::Hal>::SUPPORTS_HOME;
        if try_home {
            BootMode::ConfiguratorHome
        } else {
            BootMode::ConfiguratorField
        }
    };

    reset.reboot_into(mode).await;
}
