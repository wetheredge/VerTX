use alloc::vec::Vec;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel;
use vertx_backpack_ipc::ToMain;

pub(crate) type ResponseChannel = channel::Channel<NoopRawMutex, Vec<u8>, 10>;
pub(crate) type ResponseSender = channel::Sender<'static, NoopRawMutex, Vec<u8>, 10>;
pub(crate) type ResponseReceiver = channel::Receiver<'static, NoopRawMutex, Vec<u8>, 10>;

pub(crate) struct Api {
    tx: crate::ipc::TxSender,
    rx: ResponseReceiver,
}

impl Api {
    pub(crate) fn new(tx: crate::ipc::TxSender, rx: ResponseReceiver) -> Self {
        Self { tx, rx }
    }
}

impl vertx_server::Api for Api {
    type Buffer = Option<Vec<u8>>;

    const NAME: &'static str = "v0";

    fn buffer() -> Self::Buffer {
        None
    }

    async fn next_response<'b>(&self, buffer: &'b mut Self::Buffer) -> &'b [u8] {
        let response = self.rx.receive().await;
        buffer.insert(response)
    }

    async fn handle<'b>(&self, request: &[u8], _buffer: &'b mut Self::Buffer) -> Option<&'b [u8]> {
        self.tx
            .send(ToMain::ApiRequest(request.to_vec().into()))
            .await;
        None
    }
}
