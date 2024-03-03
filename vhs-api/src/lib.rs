#![no_std]
#![cfg_attr(not(any(feature = "embassy", feature = "tokio")), allow(unused))]

mod protocol;

use core::future::Future;

use bincode::error::EncodeError;
use embassy_futures::select;
use picoserve::extract::FromRequest;
use picoserve::io;
use picoserve::response::{ws, IntoResponse, Response as HttpResponse, StatusCode};
use picoserve::routing::MethodHandler;

pub use self::protocol::{response, Request, Response};

pub trait State {
    const BUILD_INFO: response::BuildInfo;

    fn status(&self) -> impl Future<Output = response::Status>;
    fn power_off(&self) -> !;
    fn reboot(&self) -> !;
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
                    .await;
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
    response_buffer: [u8; 64],
    state: &'a S,
}

impl<'a, S> Handler<'a, S> {
    pub fn new(state: &'a S) -> Self {
        Self {
            response_buffer: core::array::from_fn(|_| 0),
            state,
        }
    }
}

impl<S> Handler<'_, S> {
    async fn send<W: io::Write>(
        &mut self,
        response: Response,
        tx: &mut ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        let buffer = &mut self.response_buffer;

        match bincode::encode_into_slice(response, buffer, BINCODE_CONFIG) {
            Ok(len) => tx.send_binary(&buffer[0..len]).await,
            Err(err) => {
                log::error!("Failed to encode api response: {err}");
                Ok(())
            }
        }
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
            let status = self.state.status();
            let request = rx.next_message(&mut req_buffer);

            match select::select(status, request).await {
                select::Either::First(status) => self.send::<W>(status.into(), &mut tx).await?,

                select::Either::Second(request) => match request {
                    Ok(Message::Binary(request)) => {
                        let request = match bincode::decode_from_slice(request, BINCODE_CONFIG) {
                            Ok((request, _)) => request,
                            Err(err) => {
                                log::error!("Failed to parse request: {err:?}");
                                continue;
                            }
                        };

                        log::debug!("Received api request: {request:?}");

                        let response = match request {
                            Request::ProtocolVersion => Response::PROTOCOL_VERSION,
                            Request::BuildInfo => S::BUILD_INFO.into(),
                            Request::PowerOff => self.state.power_off(),
                            Request::Reboot => self.state.reboot(),
                            Request::CheckForUpdate => todo!(),
                            Request::StreamInputs => todo!(),
                            Request::StreamMixer => todo!(),
                        };

                        self.send::<W>(response, &mut tx).await?;
                    }

                    Ok(Message::Ping(payload)) => tx.send_pong(payload).await?,
                    Ok(Message::Close(close)) => break tx.close(close).await,

                    Ok(Message::Text(_)) | Ok(Message::Pong(_)) => continue,
                    Err(err) => {
                        log::error!("WebSocket error: {err:?}");
                        continue;
                    }
                },
            }
        }
    }
}
