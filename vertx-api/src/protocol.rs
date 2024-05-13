use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

pub(crate) const VERSION_MAJOR: u8 = 0;
pub(crate) const VERSION_MINOR: u8 = 0;
pub(crate) const NAME: &str = "v0";

#[derive(Debug, Clone, Deserialize)]
pub enum Request<'a> {
    ProtocolVersion,
    BuildInfo,
    PowerOff,
    Reboot,
    ExitConfigurator,
    CheckForUpdate,
    GetConfig,
    ConfigUpdate {
        id: u32,
        key: &'a str,
        value: vertx_config::update::Update<'a>,
    },
    // StreamInputs,
    // StreamMixer,
}

macro_rules! response {
    (
        $derive:path,
        $(
            $variant:ident $({
                $( $(#[$fattr:meta])* $field:ident : $type:ty ),*
                $(,)?
            })?
        ),*
        $(,)?
    ) => {
        #[derive(Debug, Clone, $derive)]
        pub enum Response {$(
            $variant $({
                $( $(#[$fattr])* $field : $type),*
            })?
        ),*}

        pub mod response { $($(
            use super::*;

            #[derive(Debug, Clone)]
            pub struct $variant {
                $( $(#[$fattr])* pub $field : $type ),*
            }

            impl From<$variant> for super::Response {
                fn from(value: $variant) -> Self {
                    Self::$variant {
                        $( $field : value.$field ),*
                    }
                }
            }
        )?)* }
    };
}

response! {
    Serialize,
    ProtocolVersion {
        major: u8,
        minor: u8,
    },
    BuildInfo {
        target: &'static str,
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
        /// Per cell battery voltage in centivolts
        battery_voltage: u16,
        idle_time: f32,
        timing_drift: f32,
    },
    Config {
        config: Vec<u8>,
    },
    ConfigUpdate {
        id: u32,
        result: ConfigUpdateResult,
    },
    // Inputs,
    // Mixer,
}

impl Response {
    pub const PROTOCOL_VERSION: Self = Self::ProtocolVersion {
        major: VERSION_MAJOR,
        minor: VERSION_MINOR,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum ConfigUpdateResult {
    Ok,
    KeyNotFound,
    InvalidType,
    InvalidValue,
    TooSmall { min: i64 },
    TooLarge { max: i64 },
}

impl From<vertx_config::update::Result> for ConfigUpdateResult {
    fn from(from: vertx_config::update::Result) -> Self {
        use vertx_config::update::Error;

        match from {
            Ok(()) => Self::Ok,
            Err(Error::KeyNotFound) => Self::KeyNotFound,
            Err(Error::InvalidType) => Self::InvalidType,
            Err(Error::InvalidValue) => Self::InvalidValue,
            Err(Error::TooSmall { min }) => Self::TooSmall { min },
            Err(Error::TooLarge { max }) => Self::TooLarge { max },
        }
    }
}
