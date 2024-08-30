use std::borrow::Cow;

pub use postcard::{from_bytes_cobs as deserialize, to_stdvec_cobs as serialize};
use serde::{Deserialize, Serialize};

pub const EXIT_SHUT_DOWN: i32 = 10;
pub const EXIT_REBOOT: i32 = 11;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message<'a, T> {
    Backpack(Cow<'a, [u8]>),
    Simulator(T),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ToVertx {
    BackpackAck,
    ModeButtonPressed,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ToManager {
    SetBootMode(u8),
    StatusLed { r: u8, g: u8, b: u8 },
}

impl From<ToVertx> for Message<'_, ToVertx> {
    fn from(value: ToVertx) -> Self {
        Self::Simulator(value)
    }
}

impl From<ToManager> for Message<'_, ToManager> {
    fn from(value: ToManager) -> Self {
        Self::Simulator(value)
    }
}
