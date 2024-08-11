#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ToBackpack {
    InitAck,
    StartNetwork(vertx_server::Config),
    ApiResponse(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkKind {
    Home,
    Field,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ToMain {
    Init,
    StartNetworkError,
    NetworkUp,
    ApiConnect,
    ApiDisconnect,
    ApiRequest(Vec<u8>),
}
