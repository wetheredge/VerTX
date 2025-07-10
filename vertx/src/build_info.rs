#![cfg_attr(not(feature = "configurator"), expect(unused))]

pub(crate) const TARGET: &str = env!("VERTX_TARGET");
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const DEBUG: bool = include!(concat!(env!("OUT_DIR"), "/is_debug"));
pub(crate) const GIT_BRANCH: &str = include_str!(concat!(env!("OUT_DIR"), "/git_branch"));
pub(crate) const GIT_COMMIT: &str = include_str!(concat!(env!("OUT_DIR"), "/git_commit"));
