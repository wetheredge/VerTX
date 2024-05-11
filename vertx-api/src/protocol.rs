pub(crate) const VERSION_MAJOR: u8 = 0;
pub(crate) const VERSION_MINOR: u8 = 0;
pub(crate) const NAME: &str = "v0";

#[derive(Debug, Clone, bincode::BorrowDecode)]
pub enum Request<'a> {
    ProtocolVersion,
    BuildInfo,
    PowerOff,
    Reboot,
    CheckForUpdate,
    ConfigUpdate {
        id: u32,
        key: &'a str,
        value: ConfigUpdate<'a>,
    },
    // StreamInputs,
    // StreamMixer,
}

macro_rules! response {
    (
        $bincode:path,
        $(
            $variant:ident $({
                $( $(#[$fattr:meta])* $field:ident : $type:ty ),*
                $(,)?
            })?
        ),*
        $(,)?
    ) => {
        #[derive(Debug, Clone, $bincode)]
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
    bincode::Encode,
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

#[derive(Debug, Clone, bincode::BorrowDecode)]
pub enum ConfigUpdate<'a> {
    Boolean(bool),
    String(&'a str),
    Unsigned(u32),
    Signed(i32),
    Float(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bincode::Encode)]
pub enum ConfigUpdateResult {
    Ok,
    KeyNotFound,
    InvalidType,
    InvalidValue,
    TooSmall { min: i64 },
    TooLarge { max: i64 },
}
