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
    Configurator = 1,
}

impl From<u8> for BootMode {
    fn from(raw: u8) -> Self {
        match raw {
            1 => Self::Configurator,
            _ => Self::Standard,
        }
    }
}

impl BootMode {
    pub const fn is_configurator(self) -> bool {
        matches!(self, BootMode::Configurator)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Manager {
    mode: BootMode,
}

impl Manager {
    pub fn new(mode: BootMode) -> Self {
        Self { mode }
    }

    pub fn toggle_configurator(&self) {
        let mode = if self.mode.is_configurator() {
            BootMode::Standard
        } else {
            BootMode::Configurator
        };
        crate::hal::set_boot_mode(mode as u8);

        RESET.signal(Reset::Reboot);
    }

    pub fn shut_down(&self) {
        RESET.signal(Reset::ShutDown);
    }

    pub fn reboot(&self) {
        RESET.signal(Reset::Reboot);
    }
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
