use alloc::borrow::ToOwned;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel;
use embassy_sync::mutex::Mutex;
use vertx_backpack_ipc::ApiRequest;
use vertx_network::api::{Method, Response};

pub(crate) type ResponseChannel = channel::Channel<NoopRawMutex, Response<'static>, 1>;
pub(crate) type ResponseSender = channel::Sender<'static, NoopRawMutex, Response<'static>, 1>;
pub(crate) type ResponseReceiver = channel::Receiver<'static, NoopRawMutex, Response<'static>, 1>;

pub(crate) struct Api {
    ipc: &'static crate::ipc::Context,
    responses: Mutex<NoopRawMutex, ResponseReceiver>,
}

impl Api {
    pub(crate) fn new(ipc: &'static crate::ipc::Context, responses: ResponseReceiver) -> Self {
        Self {
            ipc,
            responses: Mutex::new(responses),
        }
    }
}

impl vertx_network::Api for Api {
    async fn handle(&self, path: &str, method: Method, request: &[u8]) -> Response {
        let response = self.responses.lock().await;
        let request = ApiRequest {
            path: path.to_owned().into(),
            method,
            body: request.to_owned().into(),
        };
        self.ipc.send_api_request(request).await;
        response.receive().await
    }
}
