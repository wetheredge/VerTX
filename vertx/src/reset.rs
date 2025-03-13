use embassy_executor::{Spawner, task};
use embassy_sync::signal::Signal;

use crate::hal::prelude::*;

type ResetSignal = Signal<crate::mutex::MultiCore, Kind>;

pub(crate) struct Manager {
    reset: &'static ResetSignal,
}

impl Manager {
    pub(crate) fn new(
        spawner: Spawner,
        hal: crate::hal::Reset,
        config: &'static crate::config::Manager,
    ) -> Self {
        static RESET: ResetSignal = Signal::new();
        let signal = &RESET;

        spawner.must_spawn(reset(hal, signal, config));

        Self { reset: signal }
    }

    pub(crate) fn reboot(&self) {
        self.reset.signal(Kind::Reboot);
    }

    pub(crate) fn shut_down(&self) {
        self.reset.signal(Kind::ShutDown);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Reboot,
    ShutDown,
}

#[task]
async fn reset(
    mut hal: crate::hal::Reset,
    reset: &'static ResetSignal,
    config: &'static crate::config::Manager,
) -> ! {
    let kind = reset.wait().await;

    config.save().await;

    match kind {
        Kind::Reboot => hal.reboot(),
        Kind::ShutDown => hal.shut_down(),
    }
}
