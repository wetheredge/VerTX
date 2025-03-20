use core::fmt;

use embedded_io_async::Write;

use crate::build_info;

pub(crate) struct Api {
    reset: crate::reset::Manager,
    config: crate::config::Manager,
}

impl Api {
    pub(crate) fn new(reset: crate::reset::Manager, config: crate::config::Manager) -> Self {
        Self { reset, config }
    }

    pub(crate) async fn handle<R: Request, W: WriteResponse>(
        &self,
        request: R,
        writer: W,
    ) -> Result<(), W::Error> {
        // TODO: check content-type against accept

        let method = request.method();
        match request.route() {
            "version" => {
                if method != Method::Get {
                    return writer.method_not_allowed("GET").await;
                }

                let release = if build_info::DEBUG {
                    b"false".as_slice()
                } else {
                    b"true".as_slice()
                };

                let len = 69
                    + build_info::TARGET.len()
                    + build_info::VERSION.len()
                    + release.len()
                    + build_info::GIT_BRANCH.len()
                    + build_info::GIT_COMMIT.len();

                let mut writer = writer.ok_with_len(ContentType::Json, len).await?;
                writer.write_all(br#"{"target":""#).await?;
                writer.write_all(build_info::TARGET.as_bytes()).await?;
                writer.write_all(br#"","version":""#).await?;
                writer.write_all(build_info::VERSION.as_bytes()).await?;
                writer.write_all(br#"","release":"#).await?;
                writer.write_all(release).await?;
                writer.write_all(br#","git":{"branch":""#).await?;
                writer.write_all(build_info::GIT_BRANCH.as_bytes()).await?;
                writer.write_all(br#"","commit":""#).await?;
                writer.write_all(build_info::GIT_COMMIT.as_bytes()).await?;
                writer.write_all(br#""}}"#).await?;
                writer.finish().await
            }
            "shut-down" => {
                if method != Method::Post {
                    return writer.method_not_allowed("POST").await;
                }

                writer.ok_empty().await?;
                self.reset.shut_down();
                Ok(())
            }
            "reboot" => {
                if method != Method::Post {
                    return writer.method_not_allowed("POST").await;
                }

                writer.ok_empty().await?;
                self.reset.reboot();
                Ok(())
            }
            "config" => match method {
                Method::Get => {
                    let mut buffer = [0; crate::config::BYTE_LENGTH];
                    if let Some(len) = self.config.serialize(&mut buffer).transpose().unwrap() {
                        let mut writer = writer.ok_with_len(ContentType::OctetStream, len).await?;
                        writer.write_all(&buffer[0..len]).await?;
                        writer.finish().await
                    } else {
                        writer.service_unavailable().await
                    }
                }
                Method::Post => match self.config.replace(request.body()).await {
                    Ok(()) => {
                        self.config.save().await;
                        writer.ok_empty().await
                    }
                    Err(()) => todo!(),
                },
                Method::Delete => {
                    self.config.reset().await;
                    writer.ok_empty().await
                }
            },
            _ => writer.not_found().await,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Method {
    Get,
    Post,
    Delete,
}

impl TryFrom<&str> for Method {
    type Error = ();

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        if raw.eq_ignore_ascii_case("get") {
            Ok(Self::Get)
        } else if raw.eq_ignore_ascii_case("post") {
            Ok(Self::Post)
        } else if raw.eq_ignore_ascii_case("delete") {
            Ok(Self::Delete)
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContentType {
    Json,
    OctetStream,
}

impl ContentType {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::OctetStream => "application/octet-stream",
        }
    }

    #[cfg_attr(not(feature = "network"), expect(unused))]
    pub(crate) const fn as_bytes(&self) -> &'static [u8] {
        self.as_str().as_bytes()
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub(crate) trait Request {
    fn method(&self) -> Method;
    fn route(&self) -> &str;
    fn body(&self) -> &[u8];
}

pub(crate) trait WriteResponse {
    type Error: loog::DebugFormat;
    type BodyWriter: WriteBody<Error = Self::Error>;
    type ChunkedBodyWriter: WriteChunkedBody<Error = Self::Error>;

    async fn method_not_allowed(self, allow: &'static str) -> Result<(), Self::Error>;
    async fn not_found(self) -> Result<(), Self::Error>;

    /// 503 Service Unavailable
    async fn service_unavailable(self) -> Result<(), Self::Error>;

    async fn ok_empty(self) -> Result<(), Self::Error>;

    async fn ok_with_len(
        self,
        typ: ContentType,
        len: usize,
    ) -> Result<Self::BodyWriter, Self::Error>;

    #[expect(unused)]
    async fn ok_chunked(self, typ: ContentType) -> Result<Self::ChunkedBodyWriter, Self::Error>;
}

pub(crate) trait WriteBody: Write {
    async fn finish(self) -> Result<(), Self::Error>;
}

#[expect(unused)]
pub(crate) trait WriteChunkedBody {
    type Error;

    async fn write(&mut self, chunk: &[&[u8]]) -> Result<(), Self::Error>;
    async fn finish(self) -> Result<(), Self::Error>;
}
