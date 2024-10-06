#[cfg(not(feature = "simulator"))]
include!(concat!(env!("OUT_DIR"), "/pins.rs"));

#[cfg_attr(feature = "chip-esp", path = "esp/mod.rs")]
#[cfg_attr(feature = "chip-rp", path = "rp/mod.rs")]
#[cfg_attr(feature = "simulator", path = "simulator/mod.rs")]
mod implementation;

use core::fmt::Debug;

pub(crate) use self::implementation::init;
#[cfg(not(feature = "backpack-boot-mode"))]
pub(crate) use self::implementation::set_boot_mode;

macro_rules! cfg_feature {
    ($feat:literal; $($i:item)+) => {
        $(#[cfg(feature = $feat)] $i)+
    };
}

pub(crate) type Reset = impl traits::Reset;
pub(crate) type Rng = impl rand::Rng;
pub(crate) type LedDriver = impl traits::LedDriver<Error = impl Debug>;
pub(crate) type ConfigStorage = impl traits::ConfigStorage;
pub(crate) type ModeButton = impl traits::ModeButton;

cfg_feature! {
    "network-native";
    pub(crate) type Network = impl vertx_network::Hal;
    pub(crate) type NetworkDriver = <Network as vertx_network::Hal>::Driver;
}

#[cfg(feature = "network-native")]
const _: () = {
    use vertx_network::Hal as _;
    assert!(Network::SUPPORTS_HOME || Network::SUPPORTS_FIELD);
};

cfg_feature! {
    "backpack";
    pub(crate) type BackpackTx =
        impl embedded_io_async::Write<Error = impl loog::DebugFormat + embedded_io_async::Error>;
    pub(crate) type BackpackRx =
        impl embedded_io_async::Read<Error = impl loog::DebugFormat + embedded_io_async::Error>;
}

#[cfg(feature = "backpack")]
pub(crate) struct Backpack {
    pub(crate) tx: BackpackTx,
    pub(crate) rx: BackpackRx,
}

pub(crate) struct Init {
    pub(crate) reset: Reset,
    pub(crate) rng: Rng,
    #[cfg(not(feature = "backpack-boot-mode"))]
    pub(crate) boot_mode: crate::BootMode,
    pub(crate) led_driver: LedDriver,
    pub(crate) config_storage: ConfigStorage,
    pub(crate) mode_button: ModeButton,
    #[cfg(feature = "backpack")]
    pub(crate) backpack: Backpack,
    #[cfg(feature = "network-native")]
    pub(crate) network: Network,
}

pub(crate) mod traits {
    use core::future::Future;

    use smart_leds::RGB8;

    pub(crate) trait Reset {
        fn shut_down(&mut self) -> !;
        fn reboot(&mut self) -> !;
    }

    pub(crate) trait LedDriver {
        type Error;

        async fn write(&mut self, data: &[RGB8]) -> Result<(), Self::Error>;
    }

    impl<T, E> LedDriver for T
    where
        T: smart_leds::SmartLedsWrite<Error = E, Color = RGB8>,
    {
        type Error = E;

        async fn write(&mut self, data: &[RGB8]) -> Result<(), Self::Error> {
            self.write(data.iter().copied())
        }
    }

    pub(crate) trait ConfigStorage {
        fn load<T>(&self, parse: impl FnOnce(&[u8]) -> Option<T>) -> Option<T>;
        fn save(&mut self, config: &crate::config::Manager);
    }

    pub(crate) trait ModeButton {
        fn wait_for_pressed(&mut self) -> impl Future<Output = ()>;
    }
}
