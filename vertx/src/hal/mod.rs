#[cfg(not(feature = "simulator"))]
include!(concat!(env!("OUT_DIR"), "/pins.rs"));

macro_rules! declare_hal_types {
    () => {
        pub(crate) type HalReset = impl crate::hal::traits::Reset;
        pub(crate) type HalStorageFuture = impl core::future::Future<Output = HalStorage>;
        pub(crate) type HalStorage = impl crate::storage::pal::Storage;
        pub(crate) type HalStatusLed = impl crate::hal::traits::StatusLed;
        pub(crate) type HalUi = impl crate::hal::traits::Ui;

        #[cfg(all(feature = "configurator", not(feature = "network")))]
        pub(crate) type HalConfigurator = impl crate::hal::traits::Configurator;

        #[cfg(feature = "network")]
        pub(crate) type HalNetwork = impl crate::hal::traits::Network;
    };
}

#[cfg_attr(feature = "chip-esp", path = "esp/mod.rs")]
#[cfg_attr(feature = "chip-rp", path = "rp/mod.rs")]
#[cfg_attr(feature = "simulator", path = "simulator/mod.rs")]
mod implementation;

#[cfg(all(feature = "configurator", not(feature = "network")))]
pub(crate) use implementation::HalConfigurator as Configurator;
#[cfg(feature = "network")]
pub(crate) use implementation::HalNetwork as Network;
pub(crate) use implementation::{
    HalReset as Reset, HalStatusLed as StatusLed, HalStorage as Storage,
    HalStorageFuture as StorageFuture, HalUi as Ui,
};
#[cfg(feature = "network")]
pub type NetworkDriver = <Network as traits::Network>::Driver;

pub(crate) fn init(spawner: embassy_executor::Spawner) -> Init {
    implementation::init(spawner)
}

pub(crate) struct Init {
    pub(crate) reset: Reset,
    pub(crate) status_led: StatusLed,
    pub(crate) storage: StorageFuture,
    pub(crate) ui: Ui,
    #[cfg(all(feature = "configurator", not(feature = "network")))]
    pub(crate) configurator: Configurator,
    #[cfg(feature = "network")]
    pub(crate) network: Network,
}

#[expect(unused_imports)]
pub(crate) mod prelude {
    pub(crate) use embassy_embedded_hal::SetConfig as _;

    #[cfg(all(feature = "configurator", not(feature = "network")))]
    pub(crate) use super::traits::Configurator as _;
    #[cfg(feature = "network")]
    pub(crate) use super::traits::Network as _;
    pub(crate) use super::traits::{Reset as _, StatusLed as _, Ui as _};
    pub(crate) use crate::storage::pal::{Directory as _, File as _, Storage as _};
}

pub(crate) mod traits {
    use core::fmt::Debug;

    use display_interface::DisplayError;
    use embedded_graphics::draw_target::DrawTarget;
    use embedded_graphics::pixelcolor::BinaryColor;

    #[expect(unused)]
    pub(crate) trait ConfigStorage {
        fn load<T>(&self, parse: impl FnOnce(&[u8]) -> Option<T>) -> Option<T>;
        fn save(&mut self, config: &[u8]);
    }

    pub(crate) trait StatusLed {
        type Error: Debug;
        async fn set(&mut self, red: u8, green: u8, blue: u8) -> Result<(), Self::Error>;
    }

    #[cfg(all(feature = "configurator", not(feature = "network")))]
    pub(crate) trait Configurator {
        type Request: crate::configurator::api::Request;
        type Writer: crate::configurator::api::WriteResponse;

        async fn start(&mut self);
        async fn receive(&mut self) -> (Self::Request, Self::Writer);
    }

    #[cfg(feature = "network")]
    pub(crate) trait Network {
        type Driver: embassy_net::driver::Driver;

        fn seed(&mut self) -> u64;

        async fn start(
            self,
            sta: Option<crate::network::Credentials>,
            ap: crate::network::Credentials,
        ) -> (crate::network::Kind, Self::Driver);
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
