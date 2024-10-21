mod respond;
mod router;

use core::ops;

use atoi::FromRadix10;
use embedded_io_async::{Read, Write};
use vertx_network::api::{EventHandler, Method};
use vertx_network::Api;

pub(crate) async fn run<R, W, A>(
    mut rx: R,
    mut tx: W,
    buffer: &mut [u8],
    api: &A,
) -> Result<(), R::Error>
where
    R: Read,
    W: Write<Error = R::Error>,
    A: Api,
{
    let mut buffer = Buffer::new(buffer);
    loop {
        let (raw_headers, body) = read_headers(&mut buffer, &mut rx).await?;

        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut request = httparse::Request::new(&mut headers);
        // Should always return `Ok(Status::Complete)` or an error, since we're reading
        // until the \r\n\r\n that marks the start of the body
        if let Err(err) = request.parse(raw_headers) {
            log::debug!("Bad request: {err:?}");
            respond::bad_request(&mut tx, b"Bad Request").await?;
            buffer.clear();
            return Ok(());
        }

        let close = request
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("connection"))
            .is_some_and(|h| h.value.eq_ignore_ascii_case(b"close"));

        let Ok(method) = request.method.unwrap_or_default().try_into() else {
            respond::bad_request(&mut tx, b"Bad Request").await?;
            buffer.clear();
            return Ok(());
        };
        let mut request = Request {
            headers: request.headers,
            body,

            method,
            path: request.path.unwrap_or_default(),
            content_length: None,

            reader: &mut rx,
        };
        match router::respond(&mut request, &mut tx, api).await? {
            router::Outcome::Complete => {
                if close {
                    return Ok(());
                }

                let total_len = raw_headers.len() + request.content_length();
                let mut total_read = buffer.len();
                while total_read < total_len {
                    total_read += buffer.read_from(&mut rx).await?;
                }
                // How much was read past the end of the request
                let extra = total_read - total_len;
                let next_request_offset = buffer.len() - extra;
                buffer.discard_prefix(next_request_offset);
            }
            router::Outcome::EventStream => {
                tx.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type:text/event-stream\r\nCache-Control:no-cache\r\n\r\n",
                )
                .await?;

                event_stream(&mut tx, api).await?;
            }
        }
    }
}

async fn event_stream<A: Api, W: Write>(tx: &mut W, api: &A) -> Result<(), W::Error> {
    loop {
        struct Handler<W>(W);

        impl<W: Write> EventHandler for Handler<W> {
            type Error = W::Error;

            async fn send(&mut self, data: &[u8]) -> Result<(), Self::Error> {
                for line in data.split(|&x| x == b'\n') {
                    self.0.write_all(b"data:").await?;
                    self.0.write_all(line).await?;
                    self.0.write(&[b'\n']).await?;
                }

                Ok(())
            }

            async fn send_named(&mut self, name: &[u8], data: &[u8]) -> Result<(), Self::Error> {
                self.0.write_all(b"event:").await?;
                self.0.write_all(name).await?;
                self.0.write(&[b'\n']).await?;

                self.send(data).await
            }
        }

        api.event(&mut Handler(&mut *tx)).await?;
    }
}

#[derive(Debug)]
struct Mime<'a> {
    typ: &'a str,
    subtype: &'a str,
    parameters: &'a str,
}

impl<'a> Mime<'a> {
    const fn new(typ: &'a str, subtype: &'a str, parameters: &'a str) -> Self {
        Self {
            typ,
            subtype,
            parameters,
        }
    }
}

impl Mime<'_> {
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
        stream.write(&[b'/']).await?;
        stream.write_all(self.subtype.as_bytes()).await?;

        if !self.parameters.is_empty() {
            stream.write(&[b';']).await?;
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

    fn clear(&mut self) {
        self.len = 0;
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

struct Request<'buffer, 'headers, 'body, 'reader, R> {
    headers: &'headers [httparse::Header<'buffer>],
    body: Buffer<'body>,

    method: Method,
    path: &'buffer str,
    content_length: Option<usize>,

    reader: &'reader mut R,
}

impl<'buffer, 'headers, R> Request<'buffer, 'headers, '_, '_, R> {
    const fn method(&self) -> Method {
        self.method
    }

    const fn path(&self) -> &'buffer str {
        self.path
    }

    const fn headers(&self) -> &'headers [httparse::Header<'buffer>] {
        self.headers
    }

    fn content_length(&mut self) -> usize {
        *self.content_length.get_or_insert_with(|| {
            self.headers
                .iter()
                .find(|header| header.name.eq_ignore_ascii_case("content-length"))
                .map(|header| usize::from_radix_10(header.value).0)
                .unwrap_or_default()
        })
    }
}

impl<R: Read> Request<'_, '_, '_, '_, R> {
    async fn body(&mut self) -> Result<&[u8], R::Error> {
        let content_length = self.content_length();
        while self.body.len() < content_length {
            self.body.read_from(self.reader).await?;
        }
        Ok(&self.body)
    }
}

/// Read until end of headers, returning separate header slice & partial body
async fn read_headers<'a, R: Read>(
    buffer: &'a mut Buffer<'_>,
    reader: &mut R,
) -> Result<(&'a [u8], Buffer<'a>), R::Error> {
    loop {
        let old_len = buffer.len();
        buffer.read_from(reader).await?;

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
            if buffer[i + 3] == b'\r' {
                if buffer[i + 4] == b'\n' {
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
