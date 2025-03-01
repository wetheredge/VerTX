use embedded_io_async::Write;

use super::Mime;

pub(super) static ASSETS: &[(&str, Asset)] = include!(concat!(env!("OUT_DIR"), "/assets.rs"));

#[derive(Debug)]
pub(super) struct Asset {
    pub(super) mime: Mime,
    gzipped: bool,
    content: &'static [u8],
}

impl Asset {
    pub(super) async fn write_response<W: Write>(&self, stream: &mut W) -> Result<(), W::Error> {
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
