#![no_std]

extern crate alloc;

use alloc::borrow::Cow;

use serde::{Deserialize, Serialize};
use vertx_network::api::{Method, Response as ApiResponse};

pub const BAUDRATE: u32 = 115_200;
pub const INIT: [u8; 6] = *b"VerTX\0";

#[derive(Debug, Serialize, Deserialize)]
pub enum ToBackpack<'a> {
    SetBootMode(u8),
    StartNetwork(vertx_network::Config),
    #[serde(borrow)]
    ApiResponse(ApiResponse<'a>),
    ShutDown,
    Reboot,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ToMain<'a> {
    NetworkUp,
    ApiRequest {
        path: Cow<'a, str>,
        method: Method,
        body: Cow<'a, [u8]>,
    },
    PowerAck,
}

#[derive(Debug)]
pub struct ApiRequest<'a> {
    pub path: Cow<'a, str>,
    pub method: Method,
    pub body: Cow<'a, [u8]>,
}

impl<'a> From<ApiRequest<'a>> for ToMain<'a> {
    fn from(request: ApiRequest<'a>) -> Self {
        Self::ApiRequest {
            path: request.path,
            method: request.method,
            body: request.body,
        }
    }
}
