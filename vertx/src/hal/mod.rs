#[cfg_attr(feature = "chip-esp", path = "esp/mod.rs")]
#[cfg_attr(feature = "simulator", path = "simulator.rs")]
mod implementation;

use core::fmt::Debug;

pub(crate) use self::implementation::{get_cycle_count, init, reboot, set_boot_mode, shut_down};

pub(crate) type Rng = impl traits::Rng;
pub(crate) type LedDriver =
    impl smart_leds::SmartLedsWrite<Error = impl Debug, Color = smart_leds::RGB8>;
pub(crate) type ConfigStorage = impl traits::ConfigStorage;
pub(crate) type ModeButton = impl traits::ModeButton;
pub(crate) type Network = impl vertx_network_hal::Hal;
pub(crate) type NetworkDriver = <Network as vertx_network_hal::Hal>::Driver;

const _: () = {
    use vertx_network_hal::Hal as _;
    assert!(Network::SUPPORTS_HOME || Network::SUPPORTS_FIELD);
};

pub(crate) struct Init {
    pub(crate) rng: Rng,
    pub(crate) boot_mode: crate::BootMode,
    pub(crate) led_driver: LedDriver,
    pub(crate) config_storage: ConfigStorage,
    pub(crate) mode_button: ModeButton,
    pub(crate) network: Network,
}

pub(crate) mod traits {
    use core::future::Future;

    pub(crate) trait ConfigStorage {
        fn load<T>(&self, parse: impl FnOnce(&[u8]) -> T) -> Option<T>;
        fn save(&mut self, data: &[u32]);
    }

    pub(crate) trait Rng: Clone {
        fn u32(&mut self) -> u32;

        fn u64(&mut self) -> u64 {
            (u64::from(self.u32()) << 32) | u64::from(self.u32())
        }
    }

    pub(crate) trait ModeButton {
        fn wait_for_pressed(&mut self) -> impl Future<Output = ()>;
    }
}
