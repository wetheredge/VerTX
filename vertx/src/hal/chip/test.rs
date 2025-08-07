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
struct Directory(String);
#[derive(Clone)]
struct DirectoryIter;
#[derive(Clone)]
enum DirectoryEntry {}
#[derive(Clone)]
struct File(String);

impl crate::storage::pal::Storage for Storage {
    type Directory = Directory;

    fn root(&self) -> Self::Directory {
        Directory(String::from("/"))
    }

    async fn flush(&self) -> Result<(), Self::Error> {
        loog::trace!("Flushing storage");
        Ok(())
    }
}

impl crate::storage::pal::Directory for Directory {
    type File = File;
    type Iter = DirectoryIter;

    async fn dir(&self, path: &str) -> Result<Self, Self::Error> {
        Ok(Directory(format!("{}/{path}", self.0)))
    }

    async fn file(&self, path: &str) -> Result<Self::File, Self::Error> {
        Ok(File(format!("{}/{path}", self.0)))
    }

    fn iter(&self) -> Self::Iter {
        DirectoryIter
    }
}

impl crate::storage::pal::DirectoryIter for DirectoryIter {
    type Directory = Directory;
    type Entry = DirectoryEntry;
    type File = File;

    async fn next(&mut self) -> Option<Result<Self::Entry, Self::Error>> {
        None
    }
}

impl crate::storage::pal::Entry for DirectoryEntry {
    type Directory = Directory;
    type File = File;

    fn name(&self) -> &[u8] {
        unreachable!()
    }

    fn is_file(&self) -> bool {
        unreachable!()
    }

    fn to_file(self) -> Option<Self::File> {
        unreachable!()
    }

    fn is_dir(&self) -> bool {
        unreachable!()
    }

    fn to_dir(self) -> Option<Self::Directory> {
        unreachable!()
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
    async fn truncate(&mut self) -> Result<(), Self::Error> {
        loog::trace!("Truncating {}", self.0);
        Ok(())
    }

    async fn close(self) -> Result<(), Self::Error> {
        loog::trace!("Closing {}", self.0);
        Ok(())
    }
}

impl ErrorType for Storage {
    type Error = Infallible;
}

impl ErrorType for Directory {
    type Error = Infallible;
}

impl ErrorType for DirectoryIter {
    type Error = Infallible;
}

impl ErrorType for DirectoryEntry {
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
