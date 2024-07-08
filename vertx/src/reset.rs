use embassy_executor::task;
use embassy_sync::signal::Signal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reset {
    Reboot,
    ShutDown,
}

static RESET: Signal<crate::mutex::MultiCore, Reset> = Signal::new();

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

pub fn reboot_into(mode: BootMode) {
    crate::hal::set_boot_mode(mode as u8);
    RESET.signal(Reset::Reboot);
}

pub fn shut_down() {
    RESET.signal(Reset::ShutDown);
}

pub fn reboot() {
    RESET.signal(Reset::Reboot);
}

#[task]
pub async fn reset(config: &'static crate::config::Manager) -> ! {
    let kind = RESET.wait().await;
    config.save().await;

    match kind {
        Reset::ShutDown => crate::hal::shut_down(),
        Reset::Reboot => crate::hal::reboot(),
    }
}
