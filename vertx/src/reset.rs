use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

use embassy_executor::task;
use embassy_sync::signal::Signal;
use esp_hal::macros::ram;
use esp_hal::reset;
use esp_hal::rtc_cntl::SocResetReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reset {
    Reset,
    PowerOff,
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
    mode: &'static AtomicU8,
}

impl Manager {
    /// Check boot mode and initialize a new reset manager
    ///
    /// # Safety
    ///
    /// Must be called exactly once as early as possible. Do not trigger a reset
    /// before this finishes.
    pub unsafe fn new() -> Self {
        #[cfg(debug_assertions)]
        {
            use core::sync::atomic::AtomicBool;

            // Verify that this only runs once in debug mode

            static IS_SINGLETON: AtomicBool = AtomicBool::new(true);

            if !IS_SINGLETON.swap(false, Ordering::AcqRel) {
                panic!("Cannot run configurator::IsEnabled::new() multiple times");
            }
        }

        let raw: &mut MaybeUninit<AtomicU8> = {
            // TODO: replace this with SyncUnsafeCell when it is stabilized
            struct Raw(UnsafeCell<MaybeUninit<AtomicU8>>);
            // SAFETY: only runs once so this is never available to multiple threads
            unsafe impl Sync for Raw {}

            #[ram(rtc_fast, uninitialized)]
            static RAW: Raw = Raw(UnsafeCell::new(MaybeUninit::uninit()));

            // SAFETY: this only runs once and RAW is contained in this block, so the
            // reference is unique
            unsafe { &mut *RAW.0.get() }
        };

        // Initialize on for any reset that could happen before this has run
        if !matches!(reset::get_reset_reason(), Some(SocResetReason::CoreSw)) {
            let init = AtomicU8::new(BootMode::default() as u8);
            raw.write(init);
        }

        // SAFETY: initialized by the if statement above, or the same on a previous boot
        let mode = unsafe { raw.assume_init_ref() };

        Self { mode }
    }

    pub fn current_mode(&self) -> BootMode {
        self.mode.load(Ordering::Acquire).into()
    }

    pub fn toggle_configurator(&self) {
        let mode = if self.current_mode().is_configurator() {
            BootMode::Standard
        } else {
            BootMode::Configurator
        };

        self.mode.store(mode as u8, Ordering::Release);
        RESET.signal(Reset::Reset);
    }

    pub fn power_off(&self) {
        RESET.signal(Reset::PowerOff);
    }

    pub fn reboot(&self) {
        RESET.signal(Reset::Reset);
    }
}

#[task]
pub async fn reset(config: &'static crate::config::Manager) {
    let kind = RESET.wait().await;
    config.save().await;

    if kind == Reset::PowerOff {
        panic!("Emulating power off");
    } else {
        reset::software_reset();
    }
}
