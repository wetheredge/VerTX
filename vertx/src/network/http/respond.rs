use embedded_io_async::Write;

/// 400 Bad Request
pub(super) async fn bad_request<W: Write>(response: &mut W, reason: &[u8]) -> Result<(), W::Error> {
    response
        .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Type:text/plain\r\nContent-Length:")
        .await?;
    write_int(response, reason.len()).await?;
    response.write_all(b"\r\n\r\n").await?;
    response.write_all(reason).await
}

pub(super) const NOT_FOUND: &[u8] =
    b"HTTP/1.1 404 Not Found\r\nContent-Type:text/plain\r\nContent-Length:9\r\n\r\nNot Found";

/// 405 Method Not Allowed
pub(super) async fn method_not_allowed<W: Write>(
    response: &mut W,
    allow: &'static str,
) -> Result<(), W::Error> {
    response
        .write_all(b"HTTP/1.1 405 Method Not Allowed\r\nAllow:")
        .await?;
    response.write_all(allow.as_bytes()).await?;
    response.write_all(b"\r\nContent-Length:0\r\n\r\n").await
}

/// 406 Not Acceptable
pub(super) async fn not_acceptable<W: Write>(
    response: &mut W,
    accept: &super::Mime,
) -> Result<(), W::Error> {
    response
        .write_all(b"HTTP/1.1 406 Not Acceptable\r\nContent-Type:text/plain\r\nContent-Length:")
        .await?;
    write_int(response, accept.len()).await?;
    response.write_all(b"\r\n\r\n").await?;
    accept.write(response).await
}

pub(super) async fn write_int<W: Write, I: itoa::Integer>(
    response: &mut W,
    int: I,
) -> Result<(), W::Error> {
    let mut buffer = itoa::Buffer::new();
    response.write_all(buffer.format(int).as_bytes()).await
}
