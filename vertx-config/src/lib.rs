#![no_std]

extern crate alloc;

mod boot_snapshot;
mod impls;
mod reactive;
pub mod storage;
pub mod update;

pub use vertx_config_macros::*;

pub use self::boot_snapshot::BootSnapshot;
pub use self::reactive::Reactive;
pub use self::storage::Storage;
pub use self::update::{UpdateMut, UpdateRef};

pub fn split_key(key: &str) -> (&str, &str) {
    key.split_once('.').unwrap_or((key, ""))
}
