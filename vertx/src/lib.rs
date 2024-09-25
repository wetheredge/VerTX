#![no_std]
#![feature(impl_trait_in_assoc_type)]
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

#[cfg_attr(target_arch = "wasm32", embassy_executor::main)]
pub async fn main(spawner: Spawner) {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    #[cfg(target_arch = "wasm32")]
    wasm_logger::init(wasm_logger::Config::new(loog::log::Level::max()));

    loog::info!("Starting VerTX");

    let hal::Init {
        reset,
        #[cfg(not(feature = "backpack-boot-mode"))]
        boot_mode,
        led_driver,
        config_storage,
        mode_button,
        #[cfg(feature = "backpack")]
        backpack,
        #[cfg(feature = "network-native")]
        mut rng,
        #[cfg(feature = "network-native")]
        network,
    } = hal::init(spawner);

    #[cfg(feature = "backpack")]
    let backpack = backpack::Backpack::new(spawner, backpack);
    #[cfg(feature = "backpack-boot-mode")]
    let boot_mode = {
        loog::debug!("Waiting on boot mode from backpack…");
        let mode = backpack.boot_mode().await;
        loog::debug!("Received boot mode");
        mode
    };

    let mode = make_static!(mode::Channel::new());

    let config_manager = make_static!(config::Manager::new(config_storage));
    let config = config_manager.config();

    let reset = make_static!(reset::Manager::new(
        spawner,
        reset,
        config_manager,
        #[cfg(feature = "backpack")]
        backpack.clone(),
    ));

    spawner.must_spawn(change_mode(boot_mode, reset, mode_button));
    spawner.must_spawn(leds::run(config, led_driver, mode.subscriber().unwrap()));

    if boot_mode.configurator_enabled() {
        loog::info!("Configurator enabled");
        mode.publish(Mode::PreConfigurator);

        let is_home = boot_mode == BootMode::ConfiguratorHome;

        let api = make_static!(api::Api::new(spawner, reset, config_manager));
        let network = network::run(
            spawner,
            is_home,
            config,
            api,
            #[cfg(feature = "network-native")]
            &mut rng,
            #[cfg(feature = "network-native")]
            network,
            #[cfg(feature = "network-backpack")]
            backpack,
        );
        match network.await {
            Ok(()) => mode.publish(Mode::Configurator),
            Err(network::Error::InvalidHomeConfig) => {
                reset.reboot_into(BootMode::ConfiguratorField).await;
            }
        }
    } else {
        loog::info!("Configurator disabled");
        mode.publish(Mode::Ok);
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
