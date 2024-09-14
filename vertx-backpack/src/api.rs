use alloc::vec::Vec;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel;

pub(crate) type ResponseChannel = channel::Channel<NoopRawMutex, Vec<u8>, 10>;
pub(crate) type ResponseSender = channel::Sender<'static, NoopRawMutex, Vec<u8>, 10>;
pub(crate) type ResponseReceiver = channel::Receiver<'static, NoopRawMutex, Vec<u8>, 10>;

pub(crate) struct Api {
    ipc: &'static crate::ipc::Context,
    rx: ResponseReceiver,
}

impl Api {
    pub(crate) fn new(ipc: &'static crate::ipc::Context, rx: ResponseReceiver) -> Self {
        Self { ipc, rx }
    }
}

impl vertx_network::Api for Api {
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
        self.ipc.send_api_request(request.to_vec()).await;
        None
    }
}
