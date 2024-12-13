#[cfg(not(feature = "simulator"))]
include!(concat!(env!("OUT_DIR"), "/pins.rs"));

macro_rules! declare_hal_types {
    () => {
        pub(crate) type HalReset = impl crate::hal::traits::Reset;
        pub(crate) type HalLedDriver = impl crate::hal::traits::LedDriver;
        pub(crate) type HalConfigStorage = impl crate::hal::traits::ConfigStorage;
        pub(crate) type HalUi = impl crate::hal::traits::Ui;

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
    HalConfigStorage as ConfigStorage, HalLedDriver as LedDriver, HalReset as Reset, HalUi as Ui,
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
    pub(crate) ui: Ui,
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
        ConfigStorage as _, LedDriver as _, Network as _, Reset as _, Ui as _,
    };
}

pub(crate) mod traits {
    use core::fmt::Debug;

    use display_interface::DisplayError;
    use embedded_graphics::draw_target::DrawTarget;
    use embedded_graphics::pixelcolor::BinaryColor;
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

    pub(crate) trait Ui: DrawTarget<Color = BinaryColor, Error = DisplayError> {
        async fn init(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn get_input(&mut self) -> crate::ui::Input;
        async fn flush(&mut self) -> Result<(), Self::Error>;
    }
}

#[cfg(feature = "display-ssd1306")]
mod display {
    use embedded_graphics as eg;
    use embedded_hal::i2c::I2c;
    use ssd1306::prelude::*;
    use ssd1306::{I2CDisplayInterface, Ssd1306Async};

    pub(super) const SIZE: eg::geometry::Size = eg::geometry::Size {
        width: 128,
        height: 64,
    };

    type Size = DisplaySize128x64;
    pub(super) type Driver<I> =
        Ssd1306Async<I2CInterface<I>, Size, ssd1306::mode::BufferedGraphicsModeAsync<Size>>;

    pub(super) fn new<I: I2c>(i2c: I) -> Driver<I> {
        let interface = I2CDisplayInterface::new(i2c);
        Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode()
    }

    pub(super) async fn init<D: DisplayConfigAsync>(display: &mut D) -> Result<(), D::Error> {
        display.init().await
    }
}
