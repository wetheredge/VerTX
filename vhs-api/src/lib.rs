#![no_std]
#![cfg_attr(not(any(feature = "embassy", feature = "tokio")), allow(unused))]

mod protocol;

use core::future::Future;

use embassy_futures::select;
use picoserve::extract::FromRequest;
use picoserve::io;
use picoserve::response::{ws, IntoResponse, Response as HttpResponse, StatusCode};
use picoserve::routing::MethodHandler;

pub use self::protocol::{Request, Response};

pub trait State {
    fn handle_request(&self, request: protocol::Request) -> Option<protocol::Response>;
    fn next_response(&self) -> impl Future<Output = protocol::Response>;
}

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Debug)]
pub struct UpgradeHandler;

impl<S: State, PathParameters> MethodHandler<S, PathParameters> for UpgradeHandler {
    async fn call_method_handler<
        R: io::Read,
        W: picoserve::response::ResponseWriter<Error = R::Error>,
    >(
        &self,
        state: &S,
        _path_parameters: PathParameters,
        mut request: picoserve::request::Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        let body = request.body.body();

        let upgrade = match ws::WebSocketUpgrade::from_request(state, request.parts, body).await {
            Ok(upgrade) => upgrade,
            Err(rejection) => {
                return rejection
                    .write_to(request.body.finalize().await?, response_writer)
                    .await
            }
        };

        let valid_protocol = upgrade
            .protocols()
            .is_some_and(|mut protocols| protocols.any(|p| p == protocol::NAME));

        let connection = request.body.finalize().await?;
        if valid_protocol {
            upgrade
                .on_upgrade(Handler::new(state))
                .with_protocol(protocol::NAME)
                .write_to(connection, response_writer)
                .await
        } else {
            HttpResponse::new(StatusCode::new(400), "Invalid protocol")
                .write_to(connection, response_writer)
                .await
        }
    }
}

#[derive(Debug)]
pub struct Handler<'a, S> {
    response_buffer: [u8; 20],
    state: &'a S,
}

impl<'a, S> Handler<'a, S> {
    pub fn new(state: &'a S) -> Self {
        Self {
            response_buffer: Default::default(),
            state,
        }
    }
}

impl<S> Handler<'_, S> {
    async fn send<R: io::Read, W: io::Write<Error = R::Error>>(
        &mut self,
        response: Response,
        tx: &mut ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        let buffer = &mut self.response_buffer;

        let len = bincode::encode_into_slice(response, buffer, BINCODE_CONFIG).unwrap();
        tx.send_binary(&buffer[0..len]).await
    }
}

impl<S: State> ws::WebSocketCallback for Handler<'_, S> {
    async fn run<R: io::Read, W: io::Write<Error = R::Error>>(
        mut self,
        mut rx: ws::SocketRx<R>,
        mut tx: ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        use ws::Message;

        let mut req_buffer = [0; 20];

        loop {
            let response = self.state.next_response();
            let request = rx.next_message(&mut req_buffer);

            let sent = match select::select(response, request).await {
                select::Either::First(response) => self.send::<R, W>(response, &mut tx).await,

                select::Either::Second(request) => match request {
                    Ok(Message::Binary(request)) => {
                        let request = match bincode::decode_from_slice(request, BINCODE_CONFIG) {
                            Ok((request, _)) => request,
                            Err(err) => {
                                log::error!("Failed to parse request: {err:?}");
                                continue;
                            }
                        };

                        if let Some(response) = self.state.handle_request(request) {
                            self.send::<R, W>(response, &mut tx).await
                        } else {
                            continue;
                        }
                    }

                    Ok(Message::Ping(payload)) => tx.send_pong(payload).await,
                    Ok(Message::Close(close)) => break tx.close(close).await,

                    Ok(Message::Text(_)) | Ok(Message::Pong(_)) => continue,
                    Err(err) => {
                        log::debug!("WebSocket error: {err:?}");
                        continue;
                    }
                },
            };

            sent?;
        }
    }
}
