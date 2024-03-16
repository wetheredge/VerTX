pub mod server;
pub mod wifi;

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

use embassy_executor::task;
use embedded_hal_async::digital::Wait;
use esp_hal::gpio::{self, AnyPin};
use esp_hal::macros::ram;
use esp_hal::reset;
use esp_hal::rtc_cntl::SocResetReason;

pub use self::wifi::Config as WifiConfig;

#[derive(Debug, Clone, Copy)]
pub struct IsEnabled {
    enabled: &'static AtomicBool,
}

impl IsEnabled {
    pub fn new() -> Self {
        static IS_SINGLETON: AtomicBool = AtomicBool::new(true);

        if !IS_SINGLETON.swap(false, Ordering::AcqRel) {
            panic!("Cannot run configurator::IsEnabled::new() multiple times");
        }

        // TODO: replace this with SyncUnsafeCell when it is stabilized
        struct Raw(UnsafeCell<MaybeUninit<AtomicBool>>);
        // SAFETY: the IS_SINGLETON check guarantees this only runs once and this is
        // never actually seen by multiple threads
        unsafe impl Sync for Raw {}

        #[ram(rtc_fast, uninitialized)]
        static RAW: Raw = Raw(UnsafeCell::new(MaybeUninit::uninit()));

        // Initialize on any reset other than user requested ones
        if !matches!(
            reset::get_reset_reason(),
            Some(
                SocResetReason::CoreSw | SocResetReason::CoreUsbUart | SocResetReason::CoreUsbJtag
            )
        ) {
            // SAFETY: IS_SINGLETON guarantees this can only run once, therefore this
            // mutable reference is always unique
            unsafe { (*RAW.0.get()).write(AtomicBool::new(false)) };
        }

        // SAFETY: IS_SINGLETON check guarantees this only runs once. The previous
        // &mut is contained in the scope of the if, leaving this the sole reference
        let raw = unsafe { &*RAW.0.get() };

        // SAFETY: already been initialized by the if above, or on a previous boot
        let enabled = unsafe { raw.assume_init_ref() };

        Self { enabled }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub fn toggle(&self) {
        self.enabled.fetch_xor(true, Ordering::AcqRel);
    }
}

#[task]
pub async fn toggle_button(mut button: AnyPin<gpio::Input<gpio::PullUp>>, enabled: IsEnabled) {
    button.wait_for_falling_edge().await.unwrap();
    enabled.toggle();
    reset::software_reset();
}
