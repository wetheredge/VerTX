use embassy_futures::select;
use picoserve::extract::FromRequest;
use picoserve::io;
use picoserve::request::{Path, Request};
use picoserve::response::{ws, IntoResponse, Response, StatusCode};
use picoserve::routing::PathRouterService;
use vertx_network::Api;

#[derive(Debug)]
pub struct UpgradeHandler;

impl<A: Api> PathRouterService<A> for UpgradeHandler {
    async fn call_request_handler_service<
        R: io::Read,
        W: picoserve::response::ResponseWriter<Error = R::Error>,
    >(
        &self,
        api: &A,
        _path_parameters: (),
        path: Path<'_>,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        match path.encoded() {
            "" | "/" => {
                let body = request.body_connection.body();
                let upgrade = match ws::WebSocketUpgrade::from_request(api, request.parts, body)
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
                    .is_some_and(|mut protocols| protocols.any(|p| p == A::NAME));

                let connection = request.body_connection.finalize().await?;
                if valid_protocol {
                    upgrade
                        .on_upgrade(Handler::new(api))
                        .with_protocol(A::NAME)
                        .write_to(connection, response_writer)
                        .await
                } else {
                    Response::new(StatusCode::BAD_REQUEST, "Invalid protocol")
                        .write_to(connection, response_writer)
                        .await
                }
            }
            _ => {
                let connection = request.body_connection.finalize().await?;
                Response::new(StatusCode::NOT_FOUND, "Not Found")
                    .write_to(connection, response_writer)
                    .await
            }
        }
    }
}

pub struct Handler<'a, A> {
    api: &'a A,
}

impl<'a, A> Handler<'a, A> {
    pub fn new(api: &'a A) -> Self {
        Self { api }
    }
}

impl<A: Api> ws::WebSocketCallback for Handler<'_, A> {
    async fn run<R: io::Read, W: io::Write<Error = R::Error>>(
        self,
        mut rx: ws::SocketRx<R>,
        mut tx: ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        use ws::Message;

        let mut req_buffer = [0; 64];
        let mut api_buffer = A::buffer();

        loop {
            let response = self.api.next_response(&mut api_buffer);
            let request = rx.next_message(&mut req_buffer);

            match select::select(response, request).await {
                select::Either::First(response) => tx.send_binary(response).await?,

                select::Either::Second(request) => match request {
                    Ok(Message::Binary(request)) => {
                        let Some(response) = self.api.handle(request, &mut api_buffer).await else {
                            continue;
                        };
                        tx.send_binary(response).await?;
                    }

                    Ok(Message::Ping(payload)) => tx.send_pong(payload).await?,
                    Ok(Message::Close(close)) => break tx.close(close).await,

                    Ok(Message::Text(message)) => {
                        log::debug!("Ignored text message: {message:?}");
                        continue;
                    }
                    Ok(Message::Pong(_)) => continue,
                    Err(err) => {
                        log::error!("WebSocket error: {err:?}");
                        continue;
                    }
                },
            }
        }
    }
}
