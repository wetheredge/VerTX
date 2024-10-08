use alloc::boxed::Box;

use serde::{Deserialize, Serialize};

use crate::config::UpdateError;

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
        id: u16,
        #[serde(borrow)]
        update: crate::config::Update<'a>,
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
            // this actually is used?
            #[allow(unused_imports)]
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
        config: Box<[u8]>,
    },
    ConfigUpdate {
        id: u16,
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
    TooSmall { min: i64 },
    TooLarge { max: i64 },
}

impl From<Result<(), UpdateError>> for ConfigUpdateResult {
    fn from(from: Result<(), UpdateError>) -> Self {
        match from {
            Ok(()) => Self::Ok,
            Err(UpdateError::TooSmall { min }) => Self::TooSmall { min },
            Err(UpdateError::TooLarge { max }) => Self::TooLarge { max },
        }
    }
}
