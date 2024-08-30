use core::future;

use embassy_executor::{task, Spawner};
use embassy_sync::signal::Signal;

#[cfg(feature = "backpack")]
use crate::backpack::Backpack;
use crate::hal::traits::Reset as _;

type ResetSignal = Signal<crate::mutex::MultiCore, Kind>;

pub(crate) struct Manager {
    reset: &'static ResetSignal,
    #[cfg(feature = "backpack-boot-mode")]
    backpack: &'static Backpack,
}

impl Manager {
    pub(crate) fn new(
        spawner: Spawner,
        hal: crate::hal::Reset,
        config: &'static crate::config::Manager,
        #[cfg(feature = "backpack")] backpack: &'static Backpack,
    ) -> Self {
        static RESET: ResetSignal = Signal::new();
        let signal = &RESET;

        spawner.must_spawn(reset(
            hal,
            signal,
            config,
            #[cfg(feature = "backpack")]
            backpack,
        ));

        Self {
            reset: signal,
            #[cfg(feature = "backpack-boot-mode")]
            backpack,
        }
    }

    pub(crate) async fn reboot_into(&self, mode: BootMode) -> ! {
        let mode = mode as u8;
        #[cfg(feature = "backpack-boot-mode")]
        self.backpack.set_boot_mode(mode).await;
        #[cfg(not(feature = "backpack-boot-mode"))]
        crate::hal::set_boot_mode(mode);
        self.reboot().await
    }

    pub(crate) async fn reboot(&self) -> ! {
        self.reset.signal(Kind::Reboot);
        future::pending().await
    }

    pub(crate) async fn shut_down(&self) -> ! {
        self.reset.signal(Kind::ShutDown);
        future::pending().await
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BootMode {
    #[default]
    Standard = 0,
    ConfiguratorHome = 1,
    ConfiguratorField = 2,
}

impl From<u8> for BootMode {
    fn from(raw: u8) -> Self {
        match raw {
            1 => Self::ConfiguratorHome,
            2 => Self::ConfiguratorField,
            _ => Self::Standard,
        }
    }
}

impl BootMode {
    pub const fn configurator_enabled(self) -> bool {
        matches!(
            self,
            BootMode::ConfiguratorHome | BootMode::ConfiguratorField
        )
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
    #[cfg(feature = "backpack")] backpack: &'static Backpack,
) -> ! {
    let kind = reset.wait().await;

    let config_saved = config.save();

    #[cfg(not(feature = "backpack"))]
    config_saved.await;
    #[cfg(feature = "backpack")]
    {
        use embassy_futures::join::join;
        match kind {
            Kind::Reboot => {
                join(config_saved, backpack.reboot()).await;
            }
            Kind::ShutDown => config_saved.await,
        }
    }

    match kind {
        Kind::Reboot => hal.reboot(),
        Kind::ShutDown => hal.shut_down(),
    }
}
