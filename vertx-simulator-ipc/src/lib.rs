pub use postcard::{from_bytes_cobs as deserialize, to_stdvec_cobs as serialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ToFirmware {
    ModeButtonPressed,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ToManager {
    ChangeMode(u8),
    ShutDown,
    Reboot,
    StatusLed { r: u8, g: u8, b: u8 },
}
