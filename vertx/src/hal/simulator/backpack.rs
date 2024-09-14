use std::convert::Infallible;
use std::vec::Vec;

use embassy_sync::channel;
use vertx_simulator_ipc as ipc;

use crate::hal::Backpack;

type RxReceiver = channel::Receiver<'static, crate::mutex::MultiCore, Vec<u8>, 10>;

pub(super) fn new(rx: RxReceiver) -> Backpack {
    Backpack {
        tx: Tx,
        rx: Rx::new(rx),
    }
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
        super::ipc_send(ipc::Message::Backpack(buf.into()));
        Ok(())
    }
}

pub(super) struct Rx {
    rx: RxReceiver,
    buffer: Option<(Vec<u8>, usize)>,
}

impl Rx {
    fn new(rx: RxReceiver) -> Self {
        Self { rx, buffer: None }
    }
}

impl embedded_io_async::ErrorType for Rx {
    type Error = Infallible;
}

impl embedded_io_async::Read for Rx {
    async fn read(&mut self, output: &mut [u8]) -> Result<usize, Self::Error> {
        let (input, offset) = if let Some(c) = self.buffer.take() {
            c
        } else {
            (self.rx.receive().await, 0)
        };

        let len = output.len().min(input.len() - offset);
        let end = offset + len;
        let slice = &input[offset..(offset + len)];
        output[..len].copy_from_slice(slice);

        if end < input.len() {
            self.buffer = Some((input, end));
        }

        Ok(len)
    }
}
