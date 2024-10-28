use alloc::vec::Vec;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel;
use static_cell::make_static;

type ResponseChannel = channel::Channel<NoopRawMutex, Vec<u8>, 10>;
type ResponseSender = channel::Sender<'static, NoopRawMutex, Vec<u8>, 10>;
type ResponseReceiver = channel::Receiver<'static, NoopRawMutex, Vec<u8>, 10>;

pub(crate) struct Api {
    ipc: &'static crate::ipc::Context,
    responses_tx: ResponseSender,
    responses_rx: ResponseReceiver,
}

impl Api {
    pub(crate) fn new(ipc: &'static crate::ipc::Context) -> Self {
        let responses = make_static!(ResponseChannel::new());
        Self {
            ipc,
            responses_tx: responses.sender(),
            responses_rx: responses.receiver(),
        }
    }

    pub(crate) async fn push_api_response(&self, response: Vec<u8>) {
        self.responses_tx.send(response).await;
    }
}

impl vertx_network::Api for Api {
    type Buffer = Option<Vec<u8>>;

    fn buffer() -> Self::Buffer {
        None
    }

    async fn next_response<'b>(&self, buffer: &'b mut Self::Buffer) -> &'b [u8] {
        let response = self.responses_rx.receive().await;
        buffer.insert(response)
    }

    async fn handle<'b>(&self, request: &[u8], _buffer: &'b mut Self::Buffer) -> Option<&'b [u8]> {
        self.ipc.send_api_request(request.to_vec()).await;
        None
    }
}
