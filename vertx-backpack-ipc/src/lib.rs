#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

pub const BAUDRATE: u32 = 115_200;

#[derive(Debug, Serialize, Deserialize)]
pub enum ToBackpack {
    InitAck,
    SetBootMode(u8),
    StartNetwork(vertx_network::Config),
    ApiResponse(Vec<u8>),
    Reboot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkKind {
    Home,
    Field,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ToMain {
    Init { boot_mode: u8 },
    NetworkUp,
    ApiRequest(Vec<u8>),
}
