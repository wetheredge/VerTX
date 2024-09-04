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

pub(crate) type Reset = impl traits::Reset;
pub(crate) type Rng = impl rand::Rng;
pub(crate) type LedDriver = impl traits::LedDriver<Error = impl Debug>;
pub(crate) type ConfigStorage = impl traits::ConfigStorage;
pub(crate) type ModeButton = impl traits::ModeButton;

#[cfg(feature = "network-native")]
pub(crate) type Network = impl vertx_network::Hal;
#[cfg(feature = "network-native")]
pub(crate) type NetworkDriver = <Network as vertx_network::Hal>::Driver;

#[cfg(feature = "network-native")]
const _: () = {
    use vertx_network::Hal as _;
    assert!(Network::SUPPORTS_HOME || Network::SUPPORTS_FIELD);
};

#[cfg(feature = "backpack")]
pub(crate) type BackpackTx =
    impl embedded_io_async::Write<Error = impl loog::DebugFormat + embedded_io_async::Error>;
#[cfg(feature = "backpack")]
pub(crate) type BackpackRx =
    impl embedded_io_async::Read<Error = impl loog::DebugFormat + embedded_io_async::Error>;
#[cfg(feature = "backpack")]
pub(crate) type BackpackAck = impl traits::BackpackAck;

#[cfg(feature = "backpack")]
pub(crate) struct Backpack {
    pub(crate) tx: BackpackTx,
    pub(crate) rx: BackpackRx,
    pub(crate) ack: BackpackAck,
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
        fn load<T>(&self, parse: impl FnOnce(&[u8]) -> T) -> Option<T>;
        fn save(&mut self, data: alloc::vec::Vec<u8>);
    }

    pub(crate) trait ModeButton {
        fn wait_for_pressed(&mut self) -> impl Future<Output = ()>;
    }

    #[cfg(feature = "backpack")]
    pub(crate) trait BackpackAck {
        /// Wait for backpack ack line to be pulled low.
        async fn wait(&mut self);
    }

    #[cfg(feature = "backpack")]
    impl<T> BackpackAck for T
    where
        T: embedded_hal_async::digital::Wait
            + embedded_hal::digital::ErrorType<Error = core::convert::Infallible>,
    {
        async fn wait(&mut self) {
            match self.wait_for_falling_edge().await {
                Ok(()) => {}
                Err(err) => match err {},
            };
        }
    }
}
