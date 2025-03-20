#![cfg_attr(not(feature = "configurator"), expect(unused))]

use embassy_executor::task;
use embassy_sync::signal::Signal;

use crate::hal::prelude::*;

type ResetSignal = Signal<crate::mutex::MultiCore, Kind>;

#[derive(Clone, Copy)]
pub(crate) struct Manager(&'static ResetSignal);

impl Manager {
    pub(crate) const fn new() -> Self {
        static RESET: ResetSignal = Signal::new();
        Self(&RESET)
    }

    pub(crate) fn reboot(self) {
        self.0.signal(Kind::Reboot);
    }

    pub(crate) fn shut_down(self) {
        self.0.signal(Kind::ShutDown);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Reboot,
    ShutDown,
}

#[task]
pub(crate) async fn run(
    manager: Manager,
    mut hal: crate::hal::Reset,
    config: crate::config::Manager,
    storage: crate::storage::Manager,
) -> ! {
    let kind = manager.0.wait().await;

    config.save().await;
    storage.flush_before_reset().await;

    match kind {
        Kind::Reboot => hal.reboot(),
        Kind::ShutDown => hal.shut_down(),
    }
}
