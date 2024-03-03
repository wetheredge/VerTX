mod protocol;

use embassy_futures::select;
use picoserve::extract::FromRequest;
use picoserve::io;
use picoserve::response::{ws, IntoResponse, Response as HttpResponse, StatusCode};
use picoserve::routing::MethodHandler;

pub use self::protocol::{Request, Response};
use super::State;

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Debug)]
pub struct UpgradeHandler;

impl<PathParameters> MethodHandler<State, PathParameters> for UpgradeHandler {
    async fn call_method_handler<
        R: io::Read,
        W: picoserve::response::ResponseWriter<Error = R::Error>,
    >(
        &self,
        state: &State,
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

pub struct Handler<'a> {
    response_buffer: [u8; 20],
    state: &'a State,
}

impl<'a> Handler<'a> {
    pub fn new(state: &'a State) -> Self {
        Self {
            response_buffer: Default::default(),
            state,
        }
    }

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

impl ws::WebSocketCallback for Handler<'_> {
    async fn run<R: io::Read, W: io::Write<Error = R::Error>>(
        mut self,
        mut rx: ws::SocketRx<R>,
        mut tx: ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        use ws::Message;

        let mut req_buffer = [0; 20];

        loop {
            let response = self.state.responses.receive();
            let request = rx.next_message(&mut req_buffer);

            match select::select(response, request).await {
                select::Either::First(response) => self.send::<R, W>(response, &mut tx).await?,

                select::Either::Second(request) => match request {
                    Ok(Message::Binary(request)) => {
                        let request = match bincode::decode_from_slice(request, BINCODE_CONFIG) {
                            Ok((request, _)) => request,
                            Err(err) => {
                                log::error!("Failed to parse request: {err:?}");
                                continue;
                            }
                        };

                        let response = match request {
                            Request::ProtocolVersion => Response::protocol_version(),
                            Request::BuildInfo => {
                                include!(concat!(env!("OUT_DIR"), "/build_info.rs"))
                            }
                            Request::PowerOff => todo!(),
                            Request::Reboot => todo!(),
                            Request::CheckForUpdate => todo!(),
                            Request::StreamInputs => todo!(),
                            Request::StreamMixer => todo!(),
                        };

                        self.send::<R, W>(response, &mut tx).await?;
                    }

                    Ok(Message::Ping(payload)) => tx.send_pong(payload).await?,
                    Ok(Message::Close(close)) => break tx.close(close).await,

                    Ok(Message::Text(_)) | Ok(Message::Pong(_)) => continue,
                    Err(err) => {
                        log::debug!("WebSocket error: {err:?}");
                        continue;
                    }
                },
            }
        }
    }
}
