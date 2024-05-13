#![no_std]
#![cfg_attr(not(any(feature = "embassy", feature = "tokio")), allow(unused))]

extern crate alloc;

mod protocol;

use core::future::Future;

use embassy_futures::select;
use picoserve::extract::FromRequest;
use picoserve::io;
use picoserve::request::{Path, Request as HttpRequest};
use picoserve::response::{ws, IntoResponse, Response as HttpResponse, StatusCode};
use picoserve::routing::PathRouterService;

pub use self::protocol::{response, ConfigUpdateResult, Request, Response};

pub trait State {
    const BUILD_INFO: response::BuildInfo;

    fn status(&self) -> impl Future<Output = response::Status>;
    fn power_off(&self) -> !;
    fn reboot(&self) -> !;

    fn config(&self) -> &impl vertx_config::Storage;
    fn update_config<'a>(
        &self,
        key: &'a str,
        value: vertx_config::update::Update<'a>,
    ) -> impl Future<Output = vertx_config::update::Result>;
}

#[derive(Debug)]
pub struct UpgradeHandler;

impl<S: State> PathRouterService<S> for UpgradeHandler {
    async fn call_request_handler_service<
        R: io::Read,
        W: picoserve::response::ResponseWriter<Error = R::Error>,
    >(
        &self,
        state: &S,
        _path_parameters: (),
        path: Path<'_>,
        mut request: HttpRequest<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        match path.encoded() {
            "" | "/" => {
                let body = request.body_connection.body();
                let upgrade = match ws::WebSocketUpgrade::from_request(state, request.parts, body)
                    .await
                {
                    Ok(upgrade) => upgrade,
                    Err(rejection) => {
                        return rejection
                            .write_to(request.body_connection.finalize().await?, response_writer)
                            .await;
                    }
                };

                let valid_protocol = upgrade
                    .protocols()
                    .is_some_and(|mut protocols| protocols.any(|p| p == protocol::NAME));

                let connection = request.body_connection.finalize().await?;
                if valid_protocol {
                    upgrade
                        .on_upgrade(Handler::new(state))
                        .with_protocol(protocol::NAME)
                        .write_to(connection, response_writer)
                        .await
                } else {
                    HttpResponse::new(StatusCode::BAD_REQUEST, "Invalid protocol")
                        .write_to(connection, response_writer)
                        .await
                }
            }
            _ => {
                let connection = request.body_connection.finalize().await?;
                HttpResponse::new(StatusCode::NOT_FOUND, "Not Found")
                    .write_to(connection, response_writer)
                    .await
            }
        }
    }
}

#[derive(Debug)]
pub struct Handler<'a, S> {
    response_buffer: [u8; 256],
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

        match postcard::to_slice(&response, buffer) {
            Ok(data) => tx.send_binary(data).await,
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

        let mut req_buffer = [0; 64];

        loop {
            let status = self.state.status();
            let request = rx.next_message(&mut req_buffer);

            match select::select(status, request).await {
                select::Either::First(status) => self.send::<W>(status.into(), &mut tx).await?,

                select::Either::Second(request) => match request {
                    Ok(Message::Binary(request)) => {
                        let request = match postcard::from_bytes(request) {
                            Ok(request) => request,
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
                            Request::GetConfig => {
                                let config = self.state.config();
                                let config = vertx_config::storage::postcard::to_vec(config).await;
                                Response::Config { config }
                            }
                            Request::ConfigUpdate { id, key, value } => {
                                let result = self.state.update_config(key, value).await.into();
                                Response::ConfigUpdate { id, result }
                            }
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
