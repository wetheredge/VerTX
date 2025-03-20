use core::mem;

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

pub(super) struct ResponseWriter<W>(pub(super) W);

impl<W> crate::configurator::api::WriteResponse for ResponseWriter<W>
where
    W: Write,
    W::Error: loog::DebugFormat,
{
    type BodyWriter = BodyWriter<W>;
    type ChunkedBodyWriter = ChunkedBodyWriter<W>;
    type Error = W::Error;

    async fn method_not_allowed(mut self, allow: &'static str) -> Result<(), Self::Error> {
        respond::method_not_allowed(&mut self.0, allow).await
    }

    async fn not_found(mut self) -> Result<(), Self::Error> {
        self.0.write_all(respond::NOT_FOUND).await
    }

    async fn ok_empty(mut self) -> Result<(), Self::Error> {
        self.0
            .write_all(b"HTTP/1.1 200 Ok\r\nContent-Length:0\r\n\r\n")
            .await
    }

    async fn ok_with_len(
        self,
        typ: ContentType,
        len: usize,
    ) -> Result<Self::BodyWriter, Self::Error> {
        let mut tx = self.0;
        tx.write_all(b"HTTP/1.1 200 Ok\r\nContent-Type:").await?;
        tx.write_all(typ.as_bytes()).await?;
        tx.write_all(b"\r\nContent-Length:").await?;
        respond::write_int(&mut tx, len).await?;
        tx.write_all(b"\r\n\r\n").await?;
        Ok(BodyWriter(tx))
    }

    async fn ok_chunked(self, typ: ContentType) -> Result<Self::ChunkedBodyWriter, Self::Error> {
        let mut tx = self.0;
        tx.write_all(b"HTTP/1.1 200 Ok\r\nTransfer-Encoding:chunked\r\nContent-Type:")
            .await?;
        tx.write_all(typ.as_bytes()).await?;
        tx.write_all(b"\r\n\r\n").await?;
        Ok(ChunkedBodyWriter(tx))
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

pub(super) struct ChunkedBodyWriter<W>(W);

impl<W: Write> crate::configurator::api::WriteChunkedBody for ChunkedBodyWriter<W> {
    type Error = W::Error;

    async fn write(&mut self, chunk: &[&[u8]]) -> Result<(), Self::Error> {
        let len: usize = chunk.iter().map(|x| x.len()).sum();
        if len == 0 {
            if cfg!(debug_assertions) {
                loog::panic!("tried to send zero-length chunk");
            }
            return Ok(());
        }

        let mut buffer = [0; mem::size_of::<usize>() * 2];
        // TODO: trim before hex_encode?
        let len = hex_encode(&len.to_be_bytes(), &mut buffer).unwrap();
        // len is verified to be > 0, so this can't return an empty string
        let len = len.trim_start_matches('0');

        self.0.write_all(len.as_bytes()).await?;
        self.0.write_all(b"\r\n").await?;
        for segment in chunk {
            self.0.write_all(segment).await?;
        }
        self.0.write_all(b"\r\n").await
    }

    async fn finish(mut self) -> Result<(), Self::Error> {
        self.0.write_all(b"0\r\n\r\n").await
    }
}
