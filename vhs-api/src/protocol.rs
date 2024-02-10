pub(crate) const VERSION_MAJOR: u8 = 0;
pub(crate) const VERSION_MINOR: u8 = 0;
pub(crate) const NAME: &str = "v0";

#[derive(Debug, Clone, PartialEq, Eq, Hash, bincode::Decode)]
pub enum Request {
    ProtocolVersion,
    BuildInfo,
    PowerOff,
    Reboot,
    CheckForUpdate,
    StreamInputs,
    StreamMixer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, bincode::Encode)]
pub enum Response {
    ProtocolVersion {
        major: u8,
        minor: u8,
    },
    BuildInfo {
        major: u8,
        minor: u8,
        patch: u8,
        suffix: &'static str,
        debug: bool,
        git_branch: &'static str,
        git_commit: &'static str,
        git_dirty: bool,
    },
    Status {
        // Per cell battery voltage in centivolts
        battery_voltage: u16,
    },
    // Inputs,
    // Mixer,
}

impl Response {
    pub const fn protocol_version() -> Self {
        Self::ProtocolVersion {
            major: VERSION_MAJOR,
            minor: VERSION_MINOR,
        }
    }
}
