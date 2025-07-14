#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

extern crate alloc;
#[cfg(any(test, feature = "simulator"))]
extern crate std;

mod build_info;
mod config;
#[cfg(feature = "configurator")]
mod configurator;
mod crsf;
mod hal;
mod init_counter;
mod leds;
mod mode;
mod models;
mod mutex;
#[cfg(feature = "network")]
mod network;
mod reset;
mod storage;
mod ui;
#[cfg(feature = "usb")]
mod usb;
mod utils;

use embassy_executor::Spawner;
use embassy_sync::watch::Watch;

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

    let config_manager = config::Manager::new();
    let config = config_manager.config();

    let models = models::Manager::new();

    let storage = storage::Manager::new();
    spawner.must_spawn(storage::run(
        inits,
        hal.storage,
        storage,
        config_manager,
        models,
    ));

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
        models,
        #[cfg(feature = "configurator")]
        configurator,
    ));

    #[cfg(feature = "usb")]
    let usb = usb::init(spawner, hal.usb);

    let reset = reset::Manager::new();
    spawner.must_spawn(reset::run(reset, hal.reset, config_manager, storage));

    INITS.wait().await;
    loog::info!("Initialized");
    mode_sender.send(Mode::Ok);

    #[cfg(feature = "configurator")]
    {
        configurator.wait().await;
        mode_sender.send(Mode::PreConfigurator);

        static API: static_cell::StaticCell<configurator::Api> = static_cell::StaticCell::new();
        let api = API.init_with(|| configurator::Api::new(reset, config_manager));

        #[cfg(feature = "network")]
        {
            // TODO: switch these and allow choosing
            #[cfg(feature = "network-wifi")]
            let init = network::Init::Wifi(hal.wifi);
            #[cfg(feature = "network-usb-ethernet")]
            let init = network::Init::Ethernet(usb.network);
            network::init(spawner, config, api, hal.get_network_seed, init).await;
        }
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
