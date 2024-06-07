use embassy_futures::select;
use picoserve::extract::FromRequest;
use picoserve::io;
use picoserve::request::{Path, Request as HttpRequest};
use picoserve::response::{ws, IntoResponse, Response as HttpResponse, StatusCode};
use picoserve::routing::PathRouterService;

use super::protocol::{response, Request, Response};

#[derive(Debug)]
pub struct UpgradeHandler;

impl PathRouterService<super::State> for UpgradeHandler {
    async fn call_request_handler_service<
        R: io::Read,
        W: picoserve::response::ResponseWriter<Error = R::Error>,
    >(
        &self,
        state: &super::State,
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
                    .is_some_and(|mut protocols| protocols.any(|p| p == super::protocol::NAME));

                let connection = request.body_connection.finalize().await?;
                if valid_protocol {
                    upgrade
                        .on_upgrade(Handler::new(state))
                        .with_protocol(super::protocol::NAME)
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

pub struct Handler<'a> {
    response_buffer: [u8; 256],
    state: &'a super::State,
}

impl<'a> Handler<'a> {
    pub fn new(state: &'a super::State) -> Self {
        Self {
            response_buffer: core::array::from_fn(|_| 0),
            state,
        }
    }
}

impl Handler<'_> {
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

impl Handler<'_> {
    async fn config_response(&self) -> Response {
        let config = self.state.config.config();
        let config = vertx_config::storage::postcard::to_vec(config).await;
        Response::Config { config }
    }
}

impl ws::WebSocketCallback for Handler<'_> {
    async fn run<R: io::Read, W: io::Write<Error = R::Error>>(
        mut self,
        mut rx: ws::SocketRx<R>,
        mut tx: ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        use ws::Message;

        let mut req_buffer = [0; 64];

        self.send(Response::PROTOCOL_VERSION, &mut tx).await?;
        self.send(self.config_response().await, &mut tx).await?;

        loop {
            let status = self.state.status.wait();
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
                            Request::BuildInfo => {
                                include!(concat!(env!("OUT_DIR"), "/build_info.rs")).into()
                            }
                            Request::PowerOff => {
                                self.state.reset.shut_down();
                                continue;
                            }
                            Request::Reboot => {
                                self.state.reset.reboot();
                                continue;
                            }
                            Request::ExitConfigurator => {
                                self.state.reset.toggle_configurator();
                                continue;
                            }
                            Request::CheckForUpdate => todo!(),
                            Request::GetConfig => self.config_response().await,
                            Request::ConfigUpdate { id, key, value } => {
                                use vertx_config::update::UpdateRef;
                                let result = self.state.config.update_ref(key, value).await.into();
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
