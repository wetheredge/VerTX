mod api;
mod configurator;
mod respond;

use core::ops;

use atoi::FromRadix10 as _;
use embassy_executor::{Spawner, task};
use embassy_net::Stack;
use embassy_net::tcp::TcpSocket;
use embedded_io_async::{Read, Write};

use crate::configurator::Api;

pub(super) const WORKERS: usize = 8;

pub(super) fn spawn_all(spawner: Spawner, stack: Stack<'static>, api: &'static Api) {
    for id in 0..WORKERS {
        spawner.must_spawn(run(id, stack, api));
    }
}

#[task(pool_size = WORKERS)]
async fn run(id: usize, stack: Stack<'static>, api: &'static Api) -> ! {
    const TCP_BUFFER_LEN: usize = 1024;
    const HTTP_BUFFER_LEN: usize = 2048;

    let mut rx_buffer = [0; TCP_BUFFER_LEN];
    let mut tx_buffer = [0; TCP_BUFFER_LEN];
    let mut http_buffer = [0; HTTP_BUFFER_LEN];

    loop {
        let mut tcp = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

        if let Err(err) = tcp.accept(80).await {
            loog::warn!("http({id}): Accept error: {err:?}");
            continue;
        }

        if let Err(err) = server(tcp, &mut http_buffer, api).await {
            loog::error!("http({id}): Error: {err:?}");
        }
    }
}

async fn server(
    mut tcp: TcpSocket<'_>,
    buffer: &mut [u8],
    api: &Api,
) -> Result<(), embassy_net::tcp::Error> {
    let (mut rx, mut tx) = tcp.split();

    let mut buffer = Buffer::new(buffer);
    loop {
        let (raw_headers, _body) = read_headers(&mut buffer, &mut rx).await?;
        if raw_headers.is_empty() {
            return Ok(());
        }

        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut request = httparse::Request::new(&mut headers);
        // Should always return `Ok(Status::Complete)` or an error, since we're reading
        // until the \r\n\r\n that marks the start of the body
        if let Err(err) = request.parse(raw_headers) {
            loog::debug!("Bad request: {:?}", loog::Debug2Format(&err));
            respond::bad_request(&mut tx, b"Bad Request").await?;
            return Ok(());
        }

        let is_get = request
            .method
            .is_some_and(|method| method.eq_ignore_ascii_case("get"));

        let path = request.path.unwrap_or_default();
        let path = path.strip_suffix('/').unwrap_or(path);

        let connection = request
            .headers
            .iter()
            .find_map(|h| h.name.eq_ignore_ascii_case("connection").then_some(h.value));
        let content_length = request
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("content-length"))
            .map(|h| usize::from_radix_10(h.value).0)
            .unwrap_or_default();
        let accept = request
            .headers
            .iter()
            .find_map(|h| h.name.eq_ignore_ascii_case("accept").then_some(h.value))
            .and_then(|s| core::str::from_utf8(s).ok())
            .unwrap_or("*/*");

        if !is_get {
            respond::method_not_allowed(&mut tx, "GET").await?;
        } else if path == "/api" {
            return api::run(rx, tx, api, request.headers, connection).await;
        } else if let Ok(asset) = configurator::ASSETS.binary_search_by_key(&path, |(r, _)| r) {
            let asset = &configurator::ASSETS[asset].1;
            if asset.mime.is_acceptable(accept) {
                asset.write_response(&mut tx).await?;
            } else {
                respond::not_acceptable(&mut tx, &asset.mime).await?;
            }
        } else {
            tx.write_all(respond::NOT_FOUND).await?;
        }

        if connection.is_some_and(|c| c.eq_ignore_ascii_case(b"close")) {
            return Ok(());
        }

        let total_len = raw_headers.len() + content_length;
        let mut total_read = buffer.len();
        while total_read < total_len {
            total_read += buffer.read_from(&mut rx).await?;
        }
        // How much was read past the end of the request
        let extra = total_read - total_len;
        let next_request_offset = buffer.len() - extra;
        buffer.discard_prefix(next_request_offset);
    }
}

#[derive(Debug)]
struct Mime {
    typ: &'static str,
    subtype: &'static str,
    parameters: &'static str,
}

impl Mime {
    const fn new(typ: &'static str, subtype: &'static str, parameters: &'static str) -> Self {
        Self {
            typ,
            subtype,
            parameters,
        }
    }
}

impl Mime {
    fn is_acceptable(&self, accept: &str) -> bool {
        for mime in accept.split(',') {
            let mime = mime.trim();
            if mime.starts_with("*/*") {
                return true;
            }

            let Some((typ, subtype)) = mime.split_once('/') else {
                return false;
            };
            // Strip possible q-factor weight
            let subtype = self.subtype.split_once(';').map(|x| x.0).unwrap_or(subtype);

            if typ == self.typ && (subtype == self.subtype || subtype == "*") {
                return true;
            }
        }

        false
    }

    const fn len(&self) -> usize {
        let mut params = self.parameters.len();
        if params == 0 {
            params += 1;
        }
        self.typ.len() + 1 + self.subtype.len() + params
    }

    async fn write<W: Write>(&self, stream: &mut W) -> Result<(), W::Error> {
        stream.write_all(self.typ.as_bytes()).await?;
        stream.write_all(b"/").await?;
        stream.write_all(self.subtype.as_bytes()).await?;

        if !self.parameters.is_empty() {
            stream.write_all(b";").await?;
            stream.write_all(self.parameters.as_bytes()).await?;
        }

        Ok(())
    }
}

struct Buffer<'a> {
    inner: &'a mut [u8],
    len: usize,
}

impl<'a> Buffer<'a> {
    fn new(inner: &'a mut [u8]) -> Self {
        Self { inner, len: 0 }
    }

    const fn len(&self) -> usize {
        self.len
    }

    async fn read_from<R: Read>(&mut self, reader: &mut R) -> Result<usize, R::Error> {
        let len = reader.read(&mut self.inner[self.len..]).await?;
        self.len += len;
        Ok(len)
    }

    fn discard_prefix(&mut self, prefix: usize) {
        let remaining = prefix..self.len;
        self.len = remaining.len();
        self.inner.copy_within(remaining, 0);
    }
}

impl ops::Deref for Buffer<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.inner[..self.len]
    }
}

impl ops::DerefMut for Buffer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

/// Read until end of headers, returning separate header slice & partial body
async fn read_headers<'a, R: Read>(
    buffer: &'a mut Buffer<'_>,
    reader: &mut R,
) -> Result<(&'a [u8], Buffer<'a>), R::Error> {
    loop {
        let old_len = buffer.len();
        if buffer.read_from(reader).await? == 0 {
            return Ok((&[], Buffer::new(buffer)));
        }

        if let Some(body_offset) = find_body(buffer, old_len) {
            let (head, tail) = buffer.split_at_mut(body_offset);
            return Ok((head, Buffer::new(tail)));
        }
    }
}

/// Get the offset to the first byte of the body by looking for `\r\n\r\n`.
///
/// `old_len` is the length of `buffer` before the last read into it.
fn find_body(buffer: &[u8], old_len: usize) -> Option<usize> {
    // Start 3 bytes before the start of the latest chunk in case it was split
    // across the chunk boundary
    let start = old_len.saturating_sub(3);
    // Stop searching for the first byte once the rest of the sequence can no longer
    // fit in the remaining portion of `buffer
    let end = buffer.len().saturating_sub(3);

    let mut i = start;
    while i < end {
        if buffer[i..i + 2] == *b"\r\n" {
            if buffer[i + 2] == b'\r' {
                if buffer[i + 3] == b'\n' {
                    return Some(i + 4);
                }
            } else {
                // buffer[i..] is either [\r, \n, !=\r] or [\r, \n, \r, !=\n], so the next
                // initial \r can't be for at least another 3 bytes
                i += 3;
                continue;
            }
        }

        i += 1;
    }

    None
}
