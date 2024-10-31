use alloc::boxed::Box;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub(super) enum Request<'a> {
    BuildInfo,
    PowerOff,
    Reboot,
    ExitConfigurator,
    Config,
    UpdateConfig {
        id: u16,
        #[serde(borrow)]
        update: crate::config::Update<'a>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub(super) enum Response {
    BuildInfo {
        target: &'static str,
        version: &'static str,
        debug: bool,
        git_branch: &'static str,
        git_commit: &'static str,
        git_dirty: bool,
    },
    /// Per cell voltage in centivolts
    Vbat(u16),
    Config(Box<[u8]>),
    ConfigUpdate {
        id: u16,
        result: ConfigUpdateResult,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) enum ConfigUpdateResult {
    Ok,
    TooLarge { max: i64 },
    TooSmall { min: i64 },
}

impl From<Result<(), crate::config::UpdateError>> for ConfigUpdateResult {
    fn from(value: Result<(), crate::config::UpdateError>) -> Self {
        use crate::config::UpdateError;

        match value {
            Ok(()) => Self::Ok,
            Err(UpdateError::TooLarge { max }) => Self::TooLarge { max },
            Err(UpdateError::TooSmall { min }) => Self::TooSmall { min },
        }
    }
}
