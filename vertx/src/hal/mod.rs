macro_rules! select_mod {
    ($($feat:literal => $mod:ident),+ $(,)?) => {$(
        #[cfg(feature = $feat)]
        mod $mod;
        #[cfg(feature = $feat)]
        #[allow(unused_imports)]
        pub(crate) use $mod::*;
    )+};
    (
        $($feat:literal => $mod:ident,)+
        test => $test:ident $(,)?
    ) => {
        $(
            #[cfg(feature = $feat)]
            mod $mod;
            #[cfg(all(feature = $feat, not(test)))]
            #[allow(unused_imports)]
            pub(crate) use $mod::*;
        )+
        #[cfg(test)]
        mod $test;
        #[cfg(test)]
        #[allow(unused_imports)]
        pub(crate) use $test::*;
    };
}

mod display;

#[cfg(not(any(test, feature = "simulator")))]
include!(concat!(env!("OUT_DIR"), "/pins.rs"));

pub(crate) type Reset = impl crate::hal::traits::Reset;
pub(crate) type StorageFuture = impl core::future::Future<Output = Storage>;
pub(crate) type Storage =
    impl crate::storage::pal::Storage<Error = impl loog::DebugFormat + embedded_io_async::Error>;
pub(crate) type StatusLed = impl crate::hal::traits::StatusLed;
pub(crate) type Ui = impl crate::hal::traits::Ui;
#[cfg(all(feature = "configurator", not(feature = "network")))]
pub(crate) type Configurator = impl crate::hal::traits::Configurator;
#[cfg(feature = "network")]
pub(crate) type Network = impl crate::hal::traits::Network;
#[cfg(feature = "network")]
pub(crate) type NetworkDriver = <Network as traits::Network>::Driver;

mod chip;

pub(crate) fn init(spawner: embassy_executor::Spawner) -> Init {
    chip::init(spawner)
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
    pub(crate) use embedded_io_async::{Read as _, Seek as _, Write as _};

    #[cfg(all(feature = "configurator", not(feature = "network")))]
    pub(crate) use super::traits::Configurator as _;
    #[cfg(feature = "network")]
    pub(crate) use super::traits::Network as _;
    pub(crate) use super::traits::{Reset as _, StatusLed as _, Ui as _};
    pub(crate) use crate::storage::pal::{File as _, Storage as _};
}

pub(crate) mod traits {
    use core::fmt::Debug;

    use display_interface::DisplayError;
    use embedded_graphics::draw_target::DrawTarget;
    use embedded_graphics::pixelcolor::BinaryColor;

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
