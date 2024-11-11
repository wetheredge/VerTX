#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

extern crate alloc;
#[cfg(feature = "simulator")]
extern crate std;

mod api;
#[cfg(feature = "backpack")]
mod backpack;
mod build_info;
mod config;
mod crsf;
mod hal;
mod leds;
mod mode;
mod mutex;
mod network;
mod reset;

use embassy_executor::{task, Spawner};
use static_cell::StaticCell;

use crate::config::RootConfig as Config;
pub(crate) use crate::mode::Mode;
use crate::reset::BootMode;

pub async fn main(spawner: Spawner) {
    loog::info!("Starting VerTX");

    let hal = hal::init(spawner);

    #[cfg(feature = "backpack")]
    let backpack = backpack::Backpack::new(spawner, hal.backpack);

    static MODE: mode::Channel = mode::Channel::new();
    let mode = &MODE;

    static CONFIG_MANAGER: StaticCell<config::Manager> = StaticCell::new();
    let config_manager = CONFIG_MANAGER.init_with(|| config::Manager::load(hal.config_storage));
    let config = config_manager.config();

    spawner.must_spawn(leds::run(
        config,
        hal.led_driver,
        mode.subscriber().unwrap(),
    ));

    static RESET: StaticCell<reset::Manager> = StaticCell::new();
    let reset = RESET.init_with(|| {
        reset::Manager::new(
            spawner,
            hal.reset,
            config_manager,
            #[cfg(feature = "backpack")]
            backpack.clone(),
        )
    });

    #[cfg(not(feature = "backpack-boot-mode"))]
    let boot_mode = hal.boot_mode;
    #[cfg(feature = "backpack-boot-mode")]
    let boot_mode = {
        loog::debug!("Waiting on boot mode from backpack…");
        let mode = backpack.boot_mode().await;
        loog::debug!("Received boot mode");
        mode
    };

    spawner.must_spawn(change_mode(boot_mode, reset, hal.mode_button));

    if boot_mode.configurator_enabled() {
        loog::info!("Configurator enabled");
        mode.publish(Mode::PreConfigurator);

        let is_home = boot_mode == BootMode::ConfiguratorHome;

        static API: StaticCell<api::Api> = StaticCell::new();
        let api = API.init_with(|| api::Api::new(spawner, reset, config_manager));
        let network_running = network::run(
            spawner,
            is_home,
            config,
            api,
            #[cfg(feature = "network-native")]
            hal.network,
            #[cfg(feature = "network-backpack")]
            backpack,
        );

        match network_running.await {
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
        let try_home = <hal::NetworkHal as vertx_network::Hal>::SUPPORTS_HOME;
        if try_home {
            BootMode::ConfiguratorHome
        } else {
            BootMode::ConfiguratorField
        }
    };

    reset.reboot_into(mode).await;
}

#[cfg(all(feature = "simulator", target_arch = "wasm32"))]
mod simulator {
    #[global_allocator]
    /// SAFETY: The runtime environment must be single-threaded WASM.
    static ALLOCATOR: talc::TalckWasm = unsafe { talc::TalckWasm::new_global() };

    #[embassy_executor::main]
    async fn main(spawner: embassy_executor::Spawner) {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::new(loog::log::Level::max()));

        super::main(spawner).await;
    }
}
