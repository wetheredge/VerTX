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
mod ui;
mod utils;

use embassy_executor::Spawner;
use embassy_sync::watch::Watch;
use static_cell::StaticCell;

use crate::config::RootConfig as Config;
pub(crate) use crate::mode::Mode;

pub async fn main(spawner: Spawner) {
    loog::info!("Starting VerTX");

    let hal = hal::init(spawner);

    #[cfg(feature = "backpack")]
    let backpack = backpack::Backpack::new(spawner, hal.backpack);

    static MODE: crate::mode::Watch = Watch::new();
    let mode = &MODE;
    let mode_sender = mode.sender();

    static CONFIG_MANAGER: StaticCell<config::Manager> = StaticCell::new();
    let config_manager = CONFIG_MANAGER.init_with(|| config::Manager::load(hal.config_storage));
    let config = config_manager.config();

    spawner.must_spawn(leds::run(config, hal.status_led, mode.receiver().unwrap()));
    spawner.must_spawn(ui::run(config, hal.ui));

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

    loog::info!("Initialized");
    mode_sender.send(Mode::Ok);

    network::START.wait().await;
    mode_sender.send(Mode::PreConfigurator);

    static API: StaticCell<api::Api> = StaticCell::new();
    let api = API.init_with(|| api::Api::new(spawner, reset, config_manager));

    network::init(
        spawner,
        config,
        api,
        #[cfg(feature = "network-native")]
        hal.network,
        #[cfg(feature = "network-backpack")]
        backpack,
    )
    .await;

    mode_sender.send(Mode::Configurator);
    loog::info!("Network running");
}

#[cfg(all(feature = "simulator", target_arch = "wasm32"))]
mod simulator {
    #[embassy_executor::main]
    async fn main(spawner: embassy_executor::Spawner) {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::new(loog::log::Level::max()));

        super::main(spawner).await;
    }
}
