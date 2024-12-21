use embassy_executor::{task, Spawner};
use embassy_sync::signal::Signal;

#[cfg(feature = "backpack")]
use crate::backpack::Backpack;
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
        #[cfg(feature = "backpack")] backpack: Backpack,
    ) -> Self {
        static RESET: ResetSignal = Signal::new();
        let signal = &RESET;

        spawner.must_spawn(reset(
            hal,
            signal,
            config,
            #[cfg(feature = "backpack")]
            backpack.clone(),
        ));

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
    #[cfg(feature = "backpack")] backpack: Backpack,
) -> ! {
    let kind = reset.wait().await;

    let config_saved = config.save();

    #[cfg(not(feature = "backpack"))]
    config_saved.await;
    #[cfg(feature = "backpack")]
    {
        use embassy_futures::join::join;
        match kind {
            Kind::Reboot => join(config_saved, backpack.reboot()).await,
            Kind::ShutDown => join(config_saved, backpack.shut_down()).await,
        };
    }

    match kind {
        Kind::Reboot => hal.reboot(),
        Kind::ShutDown => hal.shut_down(),
    }
}
