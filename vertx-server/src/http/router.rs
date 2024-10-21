use embedded_io_async::{Read, Write};
use vertx_network::api::{Method, Response as ApiResponse};
use vertx_network::Api;

use super::{respond, Mime};

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

#[derive(Debug, Clone, Copy)]
pub(super) enum Outcome<StreamId> {
    Complete,
    EventStream(StreamId),
}

pub(super) async fn respond<R: Read, W: Write<Error = R::Error>, A: Api>(
    request: &mut super::Request<'_, '_, '_, '_, R>,
    response: &mut W,
    api: &A,
) -> Result<Outcome<A::StreamId>, W::Error> {
    let accept = request
        .headers()
        .iter()
        .find_map(|header| {
            header
                .name
                .eq_ignore_ascii_case("accept")
                .then_some(header.value)
        })
        .and_then(|s| core::str::from_utf8(s).ok())
        .unwrap_or("*/*");

    if let Some(path) = request.path().strip_prefix("/api/") {
        let method = request.method();
        let body = request.body().await?;
        match api.handle(path, method, body).await {
            ApiResponse::Ok(Some(body)) => respond::ok(response, &body.mime, &body.body).await?,
            ApiResponse::Ok(None) => respond::ok_default(response).await?,
            ApiResponse::NotFound => respond::not_found(response).await?,
            ApiResponse::BadRequest { reason } => respond::bad_request(response, &reason).await?,
            ApiResponse::MethodNotAllowed(allow) => {
                respond::method_not_allowed(response, &allow).await?;
            }
            ApiResponse::EventStream(id) => return Ok(Outcome::EventStream(id)),
        }
    } else if request.method() == Method::Get {
        let file =
            if let Ok(asset) = ASSETS.binary_search_by_key(&request.path(), |(route, _)| route) {
                &ASSETS[asset].1
            } else {
                &INDEX
            };

        if file.mime.is_acceptable(accept) {
            file.write_response(response).await?;
        } else {
            respond::not_acceptable(response, &file.mime).await?;
        }
    } else {
        respond::method_not_allowed(response, &[Method::Get]).await?;
    }

    Ok(Outcome::Complete)
}

#[derive(Debug)]
struct File {
    mime: Mime<'static>,
    gzipped: bool,
    content: &'static [u8],
}

impl File {
    async fn write_response<W: Write>(&self, stream: &mut W) -> Result<(), W::Error> {
        stream.write_all(b"HTTP/1.1 200 Ok\r\n").await?;

        stream.write_all(b"Content-Type:").await?;
        self.mime.write(stream).await?;
        stream.write_all(b"\r\n").await?;

        if self.gzipped {
            stream.write_all(b"Content-Encoding:gzip\r\n").await?;
        }

        stream.write_all(b"Content-Length:").await?;
        let mut buffer = itoa::Buffer::new();
        let len = buffer.format(self.content.len());
        stream.write_all(len.as_bytes()).await?;
        stream.write_all(b"\r\n\r\n").await?;

        stream.write_all(self.content).await
    }
}
