use std::boxed::Box;

use embassy_sync::channel::Channel;

static REQUESTS: Channel<crate::mutex::MultiCore, Box<[u8]>, 10> = Channel::new();

pub(super) fn push_request(request: Box<[u8]>) {
    REQUESTS.try_send(request).unwrap();
}

pub(super) struct Configurator;

impl crate::hal::traits::Configurator for Configurator {
    type Request = Box<[u8]>;

    async fn start(&mut self) {
        super::ipc::open_configurator();
    }

    async fn receive(&mut self) -> Self::Request {
        REQUESTS.receive().await
    }

    async fn send(&mut self, response: &[u8]) {
        super::ipc::api_tx(response);
    }
}
