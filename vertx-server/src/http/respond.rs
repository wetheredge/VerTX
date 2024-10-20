use core::future::Future;

use embedded_io_async::Write;
use vertx_network::api::Method;

/// 200 OK
pub(super) async fn ok<W: Write>(
    response: &mut W,
    mime: &[u8],
    body: &[u8],
) -> Result<(), W::Error> {
    response
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type:")
        .await?;
    response.write_all(mime).await?;
    response.write_all(b"\r\nContent-Length:").await?;
    write_len(response, body.len()).await?;
    response.write_all(b"\r\n\r\n").await?;
    response.write_all(body).await
}

/// 200 OK with default body
pub(super) fn ok_default<W: Write>(
    response: &mut W,
) -> impl Future<Output = Result<(), W::Error>> + '_ {
    ok(response, b"text/plain", b"OK")
}

/// 400 Bad Request
pub(super) async fn bad_request<W: Write>(response: &mut W, reason: &[u8]) -> Result<(), W::Error> {
    response
        .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Type:text/plain\r\nContent-Length:")
        .await?;
    write_len(response, reason.len()).await?;
    response.write_all(b"\r\n\r\n").await?;
    response.write_all(reason).await
}

/// 404 Not Found
pub(super) async fn not_found<W: Write>(response: &mut W) -> Result<(), W::Error> {
    response
        .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Type:text/plain\r\nContent-Length:9\r\n\r\nNot Found")
        .await
}

/// 405 Method Not Allowed
pub(super) async fn method_not_allowed<W: Write>(
    response: &mut W,
    allow: &[Method],
) -> Result<(), W::Error> {
    response
        .write_all(b"HTTP/1.1 405 Method Not Allowed\r\nAllow:")
        .await?;
    for (i, method) in allow.iter().enumerate() {
        if i != 0 {
            response.write(&[b',']).await?;
        }
        response.write_all(method.as_bytes()).await?;
    }
    response.write_all(b"\r\nContent-Length:0\r\n\r\n").await
}

/// 406 Not Acceptable
pub(super) async fn not_acceptable<W: Write>(
    response: &mut W,
    accept: &super::Mime<'_>,
) -> Result<(), W::Error> {
    response
        .write_all(b"HTTP/1.1 406 Not Acceptable\r\nContent-Type:text/plain\r\nContent-Length:")
        .await?;
    write_len(response, accept.len()).await?;
    response.write_all(b"\r\n\r\n").await?;
    accept.write(response).await
}

async fn write_len<W: Write>(response: &mut W, len: usize) -> Result<(), W::Error> {
    let mut buffer = itoa::Buffer::new();
    let len = buffer.format(len);
    response.write_all(len.as_bytes()).await
}
