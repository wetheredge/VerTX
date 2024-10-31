use std::convert::Infallible;

use embassy_sync::pipe::Pipe;

use crate::hal::Backpack;

pub(super) type RxPipe = Pipe<crate::mutex::MultiCore, 256>;

pub(super) fn new(rx: &'static RxPipe) -> Backpack {
    Backpack { tx: Tx, rx: Rx(rx) }
}

pub(super) struct Tx;

impl embedded_io_async::ErrorType for Tx {
    type Error = Infallible;
}

impl embedded_io_async::Write for Tx {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.write_all(buf).await?;
        Ok(buf.len())
    }

    async fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        super::ipc::backpack_tx(buf);
        Ok(())
    }
}

pub(super) struct Rx(&'static RxPipe);

impl embedded_io_async::ErrorType for Rx {
    type Error = Infallible;
}

impl embedded_io_async::Read for Rx {
    async fn read(&mut self, output: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(self.0.read(output).await)
    }
}
