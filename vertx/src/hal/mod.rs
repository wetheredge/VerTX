#[cfg(not(feature = "simulator"))]
include!(concat!(env!("OUT_DIR"), "/pins.rs"));

macro_rules! declare_hal_types {
    () => {
        pub(crate) type HalReset = impl crate::hal::traits::Reset;
        pub(crate) type HalLedDriver = impl crate::hal::traits::LedDriver;
        pub(crate) type HalConfigStorage = impl crate::hal::traits::ConfigStorage;
        pub(crate) type HalModeButton = impl crate::hal::traits::ModeButton;

        #[cfg(feature = "network-native")]
        pub(crate) type HalNetwork = impl crate::hal::traits::Network;

        #[cfg(feature = "backpack")]
        pub(crate) type HalBackpackTx = impl embedded_io_async::Write<
                Error = impl loog::DebugFormat + embedded_io_async::Error,
            >;
        #[cfg(feature = "backpack")]
        pub(crate) type HalBackpackRx = impl embedded_io_async::Read<
                Error = impl loog::DebugFormat + embedded_io_async::Error,
            >;
    };
}

#[cfg_attr(feature = "chip-esp", path = "esp/mod.rs")]
#[cfg_attr(feature = "chip-rp", path = "rp/mod.rs")]
#[cfg_attr(feature = "simulator", path = "simulator/mod.rs")]
mod implementation;

#[cfg(feature = "backpack")]
pub(crate) use implementation::HalBackpackRx as BackpackRx;
#[cfg(feature = "backpack")]
pub(crate) use implementation::HalBackpackTx as BackpackTx;
#[cfg(feature = "network-native")]
pub(crate) use implementation::HalNetwork as Network;
pub(crate) use implementation::{
    HalConfigStorage as ConfigStorage, HalLedDriver as LedDriver, HalModeButton as ModeButton,
    HalReset as Reset,
};
#[cfg(feature = "network-native")]
pub type NetworkHal = <Network as traits::Network>::Hal;
#[cfg(feature = "network-native")]
pub type NetworkDriver = <NetworkHal as vertx_network::Hal>::Driver;

pub(crate) fn init(spawner: embassy_executor::Spawner) -> Init {
    implementation::init(spawner)
}

#[cfg(not(feature = "backpack-boot-mode"))]
pub(crate) fn set_boot_mode(mode: u8) {
    implementation::set_boot_mode(mode);
}

#[cfg(feature = "network-native")]
const _: () = {
    use vertx_network::Hal as _;
    assert!(NetworkHal::SUPPORTS_HOME || NetworkHal::SUPPORTS_FIELD);
};

pub(crate) struct Init {
    pub(crate) reset: Reset,
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

#[cfg(feature = "backpack")]
pub(crate) struct Backpack {
    pub(crate) tx: BackpackTx,
    pub(crate) rx: BackpackRx,
}

#[allow(unused_imports)]
pub(crate) mod prelude {
    pub(crate) use vertx_network::Hal as _;

    pub(crate) use super::traits::{
        ConfigStorage as _, LedDriver as _, ModeButton as _, Network as _, Reset as _,
    };
}

pub(crate) mod traits {
    use core::fmt::Debug;
    use core::future::Future;

    use smart_leds::RGB8;

    pub(crate) trait ConfigStorage {
        fn load<T>(&self, parse: impl FnOnce(&[u8]) -> Option<T>) -> Option<T>;
        fn save(&mut self, config: &[u8]);
    }

    pub(crate) trait LedDriver {
        type Error: Debug;

        async fn write(&mut self, data: &[RGB8]) -> Result<(), Self::Error>;
    }

    impl<T, E> LedDriver for T
    where
        T: smart_leds::SmartLedsWrite<Error = E, Color = RGB8>,
        E: Debug,
    {
        type Error = E;

        async fn write(&mut self, data: &[RGB8]) -> Result<(), Self::Error> {
            self.write(data.iter().copied())
        }
    }

    pub(crate) trait ModeButton {
        fn wait_for_pressed(&mut self) -> impl Future<Output = ()>;
    }

    #[cfg_attr(not(feature = "network-native"), allow(dead_code))]
    pub(crate) trait Network {
        type Hal: vertx_network::Hal;

        fn seed(&mut self) -> u64;
        fn hal(self) -> Self::Hal;
    }

    pub(crate) trait Reset {
        fn shut_down(&mut self) -> !;
        fn reboot(&mut self) -> !;
    }
}
