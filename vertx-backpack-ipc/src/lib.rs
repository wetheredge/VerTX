#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

pub const BAUDRATE: u32 = 115_200;
pub const INIT: [u8; 6] = *b"VerTX\0";

#[expect(clippy::large_enum_variant)]
#[derive(Debug, Serialize, Deserialize)]
pub enum ToBackpack {
    SetBootMode(u8),
    StartNetwork(vertx_network::Config),
    ApiResponse(Vec<u8>),
    ShutDown,
    Reboot,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ToMain {
    NetworkUp,
    ApiRequest(Vec<u8>),
    PowerAck,
}
