use core::fmt;

use embedded_io_async::Write;
#[cfg(feature = "defmt")]
use loog::defmt;

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
        loog::trace!("{:?} '{=str}'", request.method(), request.route());

        // TODO: check content-type against accept

        let method = request.method();
        let route = request.route();
        let (route, query) = route.split_once('?').unwrap_or((route, ""));

        match route {
            "version" => {
                if method != Method::Get {
                    return writer.method_not_allowed("GET").await;
                }

                let release = if build_info::DEBUG {
                    b"false".as_slice()
                } else {
                    b"true".as_slice()
                };

                let bufs = &[
                    br#"{"target":""#,
                    build_info::TARGET.as_bytes(),
                    br#"","version":""#,
                    build_info::VERSION.as_bytes(),
                    br#"","release":"#,
                    release,
                    br#","git":{"branch":""#,
                    build_info::GIT_BRANCH.as_bytes(),
                    br#"","commit":""#,
                    build_info::GIT_COMMIT.as_bytes(),
                    br#""}}"#,
                ];
                write_ok_split(writer, ContentType::Json, bufs).await
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
                    let len = self.config.serialize(&mut buffer).unwrap();
                    let mut writer = writer.ok_with_len(ContentType::OctetStream, len).await?;
                    writer.write_all(&buffer[0..len]).await?;
                    writer.finish().await
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
            "model" => {
                if method != Method::Get {
                    return writer.method_not_allowed("GET").await;
                }

                let id = query
                    .split('&')
                    .filter_map(|s| s.split_once('='))
                    .find_map(|(key, value)| (key == "id").then_some(value));

                if let Some(id) = id {
                    // TODO: return actual data
                    let model = &[
                        br#"{"id":""#,
                        id.as_bytes(),
                        br#"","name":"Demo Model "#,
                        id.as_bytes(),
                        br#""}"#,
                    ];

                    write_ok_split(writer, ContentType::Json, model).await
                } else {
                    writer.not_found().await
                }
            }
            "models" => {
                if method != Method::Get {
                    return writer.method_not_allowed("GET").await;
                }

                // TODO: return actual data
                let demo =
                    br#"[{"id":"0","name":"Demo Model 0"},{"id":"1","name":"Demo Model 1"}]"#;

                let mut writer = writer.ok_with_len(ContentType::Json, demo.len()).await?;
                writer.write_all(demo).await?;
                writer.finish().await
            }
            _ => writer.not_found().await,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

    async fn write(&mut self, chunks: &[&[u8]]) -> Result<(), Self::Error>;
    async fn finish(self) -> Result<(), Self::Error>;
}

async fn write_ok_split<W: WriteResponse>(
    writer: W,
    typ: ContentType,
    bufs: &[&[u8]],
) -> Result<(), W::Error> {
    let len = bufs.iter().fold(0, |acc, buf| acc + buf.len());
    let mut writer = writer.ok_with_len(typ, len).await?;
    for buf in bufs {
        writer.write_all(buf).await?;
    }
    writer.finish().await
}
