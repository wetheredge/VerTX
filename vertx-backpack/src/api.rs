use alloc::borrow::ToOwned;
use alloc::boxed::Box;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel;
use embassy_sync::mutex::Mutex;
use static_cell::make_static;
use vertx_backpack_ipc::ApiRequest;
use vertx_network::api::{Method, Response};

type ResponseSender = channel::Sender<'static, NoopRawMutex, Response<'static>, 1>;
type ResponseReceiver = channel::Receiver<'static, NoopRawMutex, Response<'static>, 1>;

type EventName = Option<Box<[u8]>>;
type EventData = Box<[u8]>;
type Event = (EventName, EventData);

type EventSender = channel::Sender<'static, NoopRawMutex, Event, 1>;
type EventReceiver = channel::Receiver<'static, NoopRawMutex, Event, 1>;

pub(crate) struct Api {
    ipc: &'static crate::ipc::Context,
    responses_tx: ResponseSender,
    responses_rx: Mutex<NoopRawMutex, ResponseReceiver>,
    events_tx: EventSender,
    events_rx: EventReceiver,
}

impl Api {
    pub(crate) fn new(ipc: &'static crate::ipc::Context) -> Self {
        let responses = make_static!(channel::Channel::new());
        let events = make_static!(channel::Channel::new());
        Self {
            ipc,
            responses_tx: responses.sender(),
            responses_rx: Mutex::new(responses.receiver()),
            events_tx: events.sender(),
            events_rx: events.receiver(),
        }
    }

    pub(crate) async fn push_api_response(&self, response: Response<'static>) {
        self.responses_tx.send(response).await;
    }

    pub(crate) async fn push_api_event(&self, name: EventName, data: EventData) {
        self.events_tx.send((name, data)).await;
    }
}

impl vertx_network::Api for Api {
    async fn handle(&self, path: &str, method: Method, request: &[u8]) -> Response<'static> {
        let response = self.responses_rx.lock().await;
        let request = ApiRequest {
            path: path.to_owned().into(),
            method,
            body: request.to_owned().into(),
        };
        self.ipc.send_api_request(request).await;
        response.receive().await
    }

    async fn event<T: vertx_network::api::EventHandler>(
        &self,
        handler: &mut T,
    ) -> Result<(), T::Error> {
        let (name, data) = self.events_rx.receive().await;

        if let Some(name) = name {
            handler.send_named(&name, &data).await
        } else {
            handler.send(&data).await
        }
    }
}
