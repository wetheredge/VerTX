use alloc::borrow::Cow;

use serde::{Deserialize, Serialize};

#[allow(async_fn_in_trait)]
pub trait Api {
    async fn handle(&self, path: &str, method: Method, request: &[u8]) -> Response<'static>;
    async fn event<T: EventHandler>(&self, handler: &mut T) -> Result<(), T::Error>;
}

#[allow(async_fn_in_trait)]
pub trait EventHandler {
    type Error;

    async fn send(&mut self, data: &[u8]) -> Result<(), Self::Error>;
    async fn send_named(&mut self, name: &[u8], data: &[u8]) -> Result<(), Self::Error>;
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Response<'a> {
    Ok(Option<Body<'a>>),
    NotFound,
    BadRequest { reason: Cow<'a, [u8]> },
    MethodNotAllowed(Cow<'a, [Method]>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Body<'a> {
    pub mime: Cow<'a, [u8]>,
    pub body: Cow<'a, [u8]>,
}

impl<'a> Response<'a> {
    pub fn binary(body: &'a [u8]) -> Self {
        let mime = b"application/octet-stream".into();
        let body = body.into();
        Self::Ok(Some(Body { mime, body }))
    }
}

impl Response<'_> {
    pub fn json<T: serde::Serialize>(
        max_len: usize,
        value: T,
    ) -> Result<Self, serde_json_core::ser::Error> {
        let mut buffer = alloc::vec![0; max_len];
        let len = serde_json_core::to_slice(&value, &mut buffer)?;
        buffer.shrink_to(len);

        let mime = b"application/json".into();
        let body = buffer.into();
        Ok(Self::Ok(Some(Body { mime, body })))
    }

    pub fn into_owned(self) -> Response<'static> {
        match self {
            Self::Ok(Some(Body { mime, body })) => Response::Ok(Some(Body {
                mime: Cow::Owned(mime.into_owned()),
                body: Cow::Owned(body.into_owned()),
            })),
            Self::Ok(None) => Response::Ok(None),
            Self::NotFound => Response::NotFound,
            Self::BadRequest { reason } => Response::BadRequest {
                reason: Cow::Owned(reason.into_owned()),
            },
            Self::MethodNotAllowed(allow) => {
                Response::MethodNotAllowed(Cow::Owned(allow.into_owned()))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

impl Method {
    pub const fn as_bytes(&self) -> &'static [u8] {
        match self {
            Self::Get => b"GET",
            Self::Head => b"HEAD",
            Self::Post => b"POST",
            Self::Put => b"PUT",
            Self::Delete => b"DELETE",
            Self::Connect => b"CONNECT",
            Self::Options => b"OPTIONS",
            Self::Trace => b"TRACE",
            Self::Patch => b"PATCH",
        }
    }
}

impl TryFrom<&str> for Method {
    type Error = ();

    fn try_from(method: &str) -> Result<Self, Self::Error> {
        macro_rules! match_method {
            ($($s:literal => $v:ident),+ $(,)?) => {
                $(if method.eq_ignore_ascii_case($s) { Ok(Self::$v) })else+
                else {
                    Err(())
                }
            };
        }

        match_method! {
            "get" => Get,
            "head" => Head,
            "post" => Post,
            "put" => Put,
            "delete" => Delete,
            "connect" => Connect,
            "options" => Options,
            "trace" => Trace,
            "patch" => Patch,
        }
    }
}
