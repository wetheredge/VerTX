#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

extern crate alloc;
#[cfg(feature = "simulator")]
extern crate std;

mod api;
mod build_info;
mod config;
#[cfg(feature = "configurator")]
mod configurator;
mod crsf;
mod hal;
mod init_counter;
mod leds;
mod mode;
mod mutex;
#[cfg(feature = "network")]
mod network;
mod reset;
mod ui;
mod utils;

use embassy_executor::Spawner;
use embassy_sync::watch::Watch;
use static_cell::StaticCell;

use crate::config::RootConfig as Config;
pub(crate) use crate::init_counter::InitCounter;
pub(crate) use crate::mode::Mode;

pub async fn main(spawner: Spawner) {
    loog::info!("Starting VerTX");

    let hal = hal::init(spawner);

    static INITS: InitCounter = InitCounter::new();
    let inits = &INITS;

    static MODE: crate::mode::Watch = Watch::new();
    let mode = &MODE;
    let mode_sender = mode.sender();

    static CONFIG_MANAGER: StaticCell<config::Manager> = StaticCell::new();
    let config_manager = CONFIG_MANAGER.init_with(|| config::Manager::load(hal.config_storage));
    let config = config_manager.config();

    #[cfg(feature = "configurator")]
    let configurator = configurator::Manager::new();

    spawner.must_spawn(leds::run(
        inits,
        config,
        hal.status_led,
        mode.receiver().unwrap(),
    ));
    spawner.must_spawn(ui::run(
        inits,
        config,
        hal.ui,
        #[cfg(feature = "configurator")]
        configurator,
    ));

    static RESET: StaticCell<reset::Manager> = StaticCell::new();
    let reset = RESET.init_with(|| reset::Manager::new(spawner, hal.reset, config_manager));

    INITS.wait().await;
    loog::info!("Initialized");
    mode_sender.send(Mode::Ok);

    #[cfg(feature = "configurator")]
    {
        configurator.wait().await;
        mode_sender.send(Mode::PreConfigurator);

        static API: StaticCell<api::Api> = StaticCell::new();
        let api = API.init_with(|| api::Api::new(spawner, reset, config_manager));

        #[cfg(feature = "network")]
        network::init(spawner, config, api, hal.network).await;
        #[cfg(not(feature = "network"))]
        spawner.must_spawn(configurator::run(api, hal.configurator));

        mode_sender.send(Mode::Configurator);
        loog::info!("Configurator running");
    }
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
