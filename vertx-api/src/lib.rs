#![no_std]
#![cfg_attr(not(any(feature = "embassy", feature = "tokio")), allow(unused))]

extern crate alloc;

mod protocol;

use alloc::vec::Vec;
use core::future::{self, Future};
use core::ops::Deref;
use core::sync::atomic::{AtomicBool, Ordering};

use bincode::error::EncodeError;
use embassy_futures::select::{select4, Either4};
use picoserve::extract::FromRequest;
use picoserve::io;
use picoserve::response::{ws, IntoResponse, Response as HttpResponse, StatusCode};
use picoserve::routing::MethodHandler;

pub use self::protocol::{response, Request, Response};

pub const ERROR_CODE_IN_USE: u16 = 4000;

pub trait State {
    const BUILD_INFO: response::BuildInfo;

    fn api_state(&self) -> &ApiState;

    fn status(&self) -> impl Future<Output = response::Status>;
    fn inputs(&self) -> impl Future<Output = Vec<u16>>;
    fn outputs(&self) -> impl Future<Output = [u16; 16]>;
    fn power_off(&self) -> !;
    fn reboot(&self) -> !;
}

#[derive(Debug)]
pub struct ApiState {
    locked: AtomicBool,
}

impl ApiState {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    fn lock(&self) -> bool {
        !self.locked.swap(true, Ordering::AcqRel)
    }

    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

impl Default for ApiState {
    fn default() -> Self {
        Self::new()
    }
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
            if state.api_state().lock() {
                upgrade
                    .on_upgrade(Handler::new(state))
                    .with_protocol(protocol::NAME)
                    .write_to(connection, response_writer)
                    .await
            } else {
                upgrade
                    .on_upgrade(InUseHandler)
                    .with_protocol(protocol::NAME)
                    .write_to(connection, response_writer)
                    .await
            }
        } else {
            HttpResponse::new(StatusCode::new(400), "Invalid protocol")
                .write_to(connection, response_writer)
                .await
        }
    }
}

#[derive(Debug)]
struct InUseHandler;

impl ws::WebSocketCallback for InUseHandler {
    async fn run<R: io::Read, W: io::Write<Error = R::Error>>(
        self,
        rx: ws::SocketRx<R>,
        tx: ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        log::warn!("Got api connection api is in use");
        tx.close((ERROR_CODE_IN_USE, "in use")).await
    }
}

#[derive(Debug)]
pub struct Handler<'a, S> {
    response_buffer: [u8; 64],
    state: &'a S,

    stream_inputs: bool,
    stream_outputs: bool,
}

impl<'a, S> Handler<'a, S> {
    pub fn new(state: &'a S) -> Self {
        Self {
            response_buffer: core::array::from_fn(|_| 0),
            state,

            stream_inputs: false,
            stream_outputs: false,
        }
    }
}

impl<S: State> Handler<'_, S> {
    async fn maybe_inputs(&self) -> Response {
        if self.stream_inputs {
            Response::Inputs {
                inputs: self.state.inputs().await,
            }
        } else {
            future::pending().await
        }
    }

    async fn maybe_outputs(&self) -> Response {
        if self.stream_outputs {
            Response::Outputs {
                outputs: self.state.outputs().await,
            }
        } else {
            future::pending().await
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
            let inputs = self.maybe_inputs();
            let outputs = self.maybe_outputs();
            let request = rx.next_message(&mut req_buffer);

            match select4(status, inputs, outputs, request).await {
                Either4::First(status) => self.send::<W>(status.into(), &mut tx).await?,
                Either4::Second(inputs) => self.send::<W>(inputs, &mut tx).await?,
                Either4::Third(outputs) => self.send::<W>(outputs, &mut tx).await?,

                Either4::Fourth(request) => match request {
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
                            Request::StreamInputs(enable) => {
                                self.stream_inputs = enable;
                                continue;
                            }
                            Request::StreamOutputs(enable) => {
                                self.stream_outputs = enable;
                                continue;
                            }
                        };

                        self.send::<W>(response, &mut tx).await?;
                    }

                    Ok(Message::Ping(payload)) => tx.send_pong(payload).await?,
                    Ok(Message::Close(close)) => {
                        let result = tx.close(close).await;
                        self.state.api_state().unlock();
                        return result;
                    }

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
