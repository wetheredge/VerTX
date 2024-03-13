use embassy_sync::blocking_mutex::raw;

pub type SingleCore = raw::NoopRawMutex;
pub type MultiCore = raw::CriticalSectionRawMutex;
