use core::{iter, mem};

use embedded_io_async::Write;
use faster_hex::hex_encode;

use super::respond;
use crate::configurator::api::{ContentType, Method};

pub(super) struct Request<'a> {
    pub(super) method: Method,
    pub(super) route: &'a str,
    pub(super) body: &'a [u8],
}

impl crate::configurator::api::Request for Request<'_> {
    fn method(&self) -> Method {
        self.method
    }

    fn route(&self) -> &str {
        self.route
    }

    fn body(&self) -> &[u8] {
        self.body
    }
}

pub(super) struct ResponseWriter<'a, W> {
    inner: W,
    #[expect(dead_code)]
    buffer: &'a mut [u8],
}

impl<'a, W> ResponseWriter<'a, W> {
    pub(super) fn new(inner: W, buffer: &'a mut [u8]) -> Self {
        Self { inner, buffer }
    }
}

impl<'a, W> crate::configurator::api::WriteResponse for ResponseWriter<'a, W>
where
    W: Write,
    W::Error: loog::DebugFormat,
{
    type BodyWriter = BodyWriter<W>;
    type ChunkedBodyWriter = ChunkedBodyWriter<'a, W>;
    type Error = W::Error;

    async fn method_not_allowed(mut self, allow: &'static str) -> Result<(), Self::Error> {
        respond::method_not_allowed(&mut self.inner, allow).await
    }

    async fn not_found(mut self) -> Result<(), Self::Error> {
        self.inner.write_all(respond::NOT_FOUND).await
    }

    async fn ok_empty(mut self) -> Result<(), Self::Error> {
        self.inner
            .write_all(b"HTTP/1.1 200 Ok\r\nContent-Length:0\r\n\r\n")
            .await
    }

    async fn ok_with_len(
        self,
        typ: ContentType,
        len: usize,
    ) -> Result<Self::BodyWriter, Self::Error> {
        let mut tx = self.inner;
        tx.write_all(b"HTTP/1.1 200 Ok\r\nContent-Type:").await?;
        tx.write_all(typ.as_bytes()).await?;
        tx.write_all(b"\r\nContent-Length:").await?;
        respond::write_int(&mut tx, len).await?;
        tx.write_all(b"\r\n\r\n").await?;
        Ok(BodyWriter(tx))
    }

    async fn ok_chunked(self, typ: ContentType) -> Result<Self::ChunkedBodyWriter, Self::Error> {
        let Self { mut inner, buffer } = self;

        inner
            .write_all(b"HTTP/1.1 200 Ok\r\nTransfer-Encoding:chunked\r\nContent-Type:")
            .await?;
        inner.write_all(typ.as_bytes()).await?;
        inner.write_all(b"\r\n\r\n").await?;

        Ok(ChunkedBodyWriter {
            inner,
            buffer,
            buffered: 0,
        })
    }
}

pub(super) struct BodyWriter<W>(W);

impl<W: Write> embedded_io_async::ErrorType for BodyWriter<W> {
    type Error = W::Error;
}

impl<W: Write> Write for BodyWriter<W> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.write(buf).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush().await
    }

    async fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.0.write_all(buf).await
    }
}

impl<W: Write> crate::configurator::api::WriteBody for BodyWriter<W> {
    async fn finish(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg_attr(not(test), expect(dead_code))]
pub(super) struct ChunkedBodyWriter<'a, W> {
    inner: W,
    buffer: &'a mut [u8],
    buffered: usize,
}

impl<W: Write> crate::configurator::api::WriteChunkedBody for ChunkedBodyWriter<'_, W> {
    type Error = W::Error;

    async fn write(&mut self, chunks: &[&[u8]]) -> Result<(), Self::Error> {
        let len: usize = chunks.iter().map(|x| x.len()).sum();
        let len = self.buffered + len;

        if len < self.buffer.len() {
            for chunk in chunks {
                let new_len = self.buffered + chunk.len();
                self.buffer[self.buffered..new_len].copy_from_slice(chunk);
                self.buffered = new_len;
            }
            return Ok(());
        }

        write_chunk(&mut self.inner, &self.buffer[0..self.buffered], chunks).await
    }

    async fn finish(mut self) -> Result<(), Self::Error> {
        write_chunk(&mut self.inner, &self.buffer[0..self.buffered], &[]).await?;
        self.inner.write_all(b"0\r\n\r\n").await
    }
}

#[cfg_attr(not(test), expect(dead_code))]
async fn write_chunk<W: Write>(
    writer: &mut W,
    buffer: &[u8],
    rest: &[&[u8]],
) -> Result<(), W::Error> {
    let len = buffer.len() + rest.iter().map(|x| x.len()).sum::<usize>();
    if len == 0 {
        return Ok(());
    }

    let mut hex_buffer = [0; mem::size_of::<usize>() * 2];
    // TODO: trim before hex_encode?
    let len = hex_encode(&len.to_be_bytes(), &mut hex_buffer).unwrap();
    // len is verified to be > 0, so this won't return an empty string
    let len = len.trim_start_matches('0');

    writer.write_all(len.as_bytes()).await?;
    writer.write_all(b"\r\n").await?;

    for segment in iter::once(&buffer).chain(rest.iter()) {
        if !segment.is_empty() {
            writer.write_all(segment).await?;
        }
    }

    writer.write_all(b"\r\n").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configurator::api::WriteChunkedBody as _;

    #[tokio::test]
    async fn no_empty_chunks() {
        struct MockWriter {
            done: bool,
        }

        impl embedded_io_async::ErrorType for MockWriter {
            type Error = core::convert::Infallible;
        }

        impl Write for MockWriter {
            async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                if self.done {
                    panic!("Tried to write after an empty chunk: {buf:?}");
                }

                self.done = buf.is_empty();
                Ok(buf.len())
            }
        }

        let mut buffer = [0; 10];
        let mut writer = ChunkedBodyWriter {
            inner: MockWriter { done: false },
            buffer: &mut buffer,
            buffered: 0,
        };

        writer.write(&[b"test"]).await.unwrap();
    }
}
