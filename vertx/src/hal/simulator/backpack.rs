use std::convert::Infallible;
use std::vec::Vec;

use embassy_sync::channel;
use embassy_sync::signal::Signal;
use vertx_simulator_ipc as ipc;

use crate::hal::Backpack;

type RxReceiver = channel::Receiver<'static, crate::mutex::MultiCore, Vec<u8>, 10>;
pub(super) type AckSignal = Signal<crate::mutex::MultiCore, ()>;

pub(super) fn new(rx: RxReceiver, ack: &'static AckSignal) -> Backpack {
    Backpack {
        tx: Tx,
        rx: Rx::new(rx),
        ack: Ack(ack),
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
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let (cached, offset) = if let Some(c) = self.buffer.take() {
            c
        } else {
            (self.rx.receive().await, 0)
        };

        let len = buf.len().min(cached.len() - offset);
        let end = offset + len;
        let slice = &cached[offset..end];
        buf[..len].copy_from_slice(slice);

        if end < len {
            self.buffer = Some((cached, end));
        }

        Ok(len)
    }
}

pub(super) struct Ack(&'static AckSignal);

impl crate::hal::traits::BackpackAck for Ack {
    async fn wait(&mut self) {
        self.0.wait().await;
    }
}
