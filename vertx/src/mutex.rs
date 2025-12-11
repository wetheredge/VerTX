use embassy_sync::blocking_mutex::raw;

pub(crate) type SingleCore = raw::NoopRawMutex;
pub(crate) type MultiCore = raw::CriticalSectionRawMutex;
