use core::convert::Infallible;
use core::task;
use std::string::String;
use std::{format, future};

use display_interface::DisplayError;
use embedded_graphics as eg;
use embedded_io_async::ErrorType;

use crate::hal;

#[define_opaque(hal::Reset, hal::StatusLed, hal::StorageFuture, hal::Ui, hal::Network)]
pub(crate) fn init(_spawner: embassy_executor::Spawner) -> hal::Init {
    hal::Init {
        reset: Reset,
        status_led: StatusLed,
        storage: async { Storage },
        ui: Ui,
        network: Network,
    }
}

struct Reset;

impl hal::traits::Reset for Reset {
    fn shut_down(&mut self) -> ! {
        panic!("shut down")
    }

    fn reboot(&mut self) -> ! {
        panic!("reboot")
    }
}

struct StatusLed;

impl hal::traits::StatusLed for StatusLed {
    type Error = ();

    async fn set(&mut self, red: u8, green: u8, blue: u8) -> Result<(), Self::Error> {
        loog::trace!("setting status led: rgb({red}, {green}, {blue})");
        Ok(())
    }
}

struct Storage;
#[derive(Clone)]
struct File(String);

impl crate::storage::pal::Storage for Storage {
    type File<'s>
        = File
    where
        Self: 's;

    async fn read_config<'a>(&mut self, _buf: &'a mut [u8]) -> Result<&'a [u8], Self::Error> {
        loog::trace!("Reading config");
        Ok(&[])
    }

    async fn write_config(&mut self, config: &[u8]) -> Result<(), Self::Error> {
        loog::trace!("Writing {} bytes to config", config.len());
        Ok(())
    }

    async fn model_names<F>(&mut self, _f: F) -> Result<(), Self::Error>
    where
        F: FnMut(crate::models::Id, &str),
    {
        loog::trace!("Listing model names");
        Ok(())
    }

    async fn model(
        &mut self,
        id: crate::models::Id,
    ) -> Result<Option<Self::File<'_>>, Self::Error> {
        loog::trace!("Opening model {id}");
        Ok(Some(File(format!("model/{id}"))))
    }

    async fn delete_model(&mut self, id: crate::models::Id) -> Result<(), Self::Error> {
        loog::trace!("Deleting model {id}");
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        loog::trace!("Flushing storage");
        Ok(())
    }
}

impl embedded_io_async::Read for File {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        loog::trace!("Reading up to {} bytes from {}", buf.len(), self.0);
        Ok(0)
    }
}

impl embedded_io_async::Write for File {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        loog::trace!("Writing {} bytes to {}: {buf:?}", buf.len(), self.0);
        Ok(buf.len())
    }
}

impl embedded_io_async::Seek for File {
    async fn seek(&mut self, pos: embedded_io_async::SeekFrom) -> Result<u64, Self::Error> {
        loog::trace!("Seeking {}: {pos:?}", self.0);
        Ok(0)
    }
}

impl crate::storage::pal::File for File {
    async fn len(&mut self) -> u64 {
        0
    }

    async fn truncate(&mut self) -> Result<(), Self::Error> {
        loog::trace!("Truncating {}", self.0);
        Ok(())
    }
}

impl ErrorType for Storage {
    type Error = Infallible;
}

impl ErrorType for File {
    type Error = Infallible;
}

struct Ui;

impl eg::geometry::OriginDimensions for Ui {
    fn size(&self) -> eg::geometry::Size {
        eg::geometry::Size {
            width: 128,
            height: 64,
        }
    }
}

impl eg::draw_target::DrawTarget for Ui {
    type Color = eg::pixelcolor::BinaryColor;
    type Error = DisplayError;

    fn draw_iter<I>(&mut self, _pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = eg::Pixel<Self::Color>>,
    {
        Ok(())
    }
}

impl hal::traits::Ui for Ui {
    async fn get_input(&mut self) -> crate::ui::Input {
        future::pending().await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        loog::trace!("Flushing display");
        Ok(())
    }
}

struct Network;
struct NetworkDriver;
struct NetworkToken;

impl hal::traits::Network for Network {
    type Driver = NetworkDriver;

    fn seed(&mut self) -> u64 {
        // chosen by fair dice roll.
        // guaranteed to be random.
        4
    }

    async fn start(
        self,
        _sta: Option<crate::network::Credentials>,
        _ap: crate::network::Credentials,
    ) -> (crate::network::Kind, Self::Driver) {
        todo!()
    }
}

impl embassy_net::driver::Driver for NetworkDriver {
    type RxToken<'a> = NetworkToken;
    type TxToken<'a> = NetworkToken;

    fn receive(
        &mut self,
        _cx: &mut task::Context,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        None
    }

    fn transmit(&mut self, _cx: &mut task::Context) -> Option<Self::TxToken<'_>> {
        None
    }

    fn link_state(&mut self, _cx: &mut task::Context) -> embassy_net::driver::LinkState {
        embassy_net::driver::LinkState::Down
    }

    fn capabilities(&self) -> embassy_net::driver::Capabilities {
        Default::default()
    }

    fn hardware_address(&self) -> embassy_net::driver::HardwareAddress {
        embassy_net::driver::HardwareAddress::Ip
    }
}

impl embassy_net::driver::RxToken for NetworkToken {
    fn consume<R, F>(self, _f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        todo!()
    }
}

impl embassy_net::driver::TxToken for NetworkToken {
    fn consume<R, F>(self, _len: usize, _f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        todo!()
    }
}
