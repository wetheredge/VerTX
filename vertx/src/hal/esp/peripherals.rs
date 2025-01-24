use esp_hal::gpio::AnyPin;

macro_rules! peripherals {
($(#[$attr:meta])* $vis:vis struct $n:ident { $( $p:ident ),+ $(,)? }) => {
    $(#[$attr])*
    $vis struct $n {
        $( $vis $p: esp_hal::peripherals::$p, )+
        $vis gpio: Gpio,
    }

    impl $n {
        $vis fn new(p: esp_hal::peripherals::Peripherals) -> Self {
            Self {
                $($p: p.$p,)+
                gpio: Gpio { taken: 0 },
            }
        }
    }
};
}

peripherals! {
    /// Wrapper struct for `esp_hal::peripherals::Peripherals` to make getting gpios
    /// at runtime a little easier.
    #[expect(non_snake_case)]
    pub(super) struct Peripherals { I2C0, RADIO_CLK, RMT, RNG, TIMG0, TIMG1, WIFI }
}

/// Allow soundly getting gpios at runtime.
///
/// # Safety
///
/// Only create one instance of this type.
pub(super) struct Gpio {
    taken: u64,
}

impl Gpio {
    pub(super) fn take(&mut self, pin: u8) -> AnyPin {
        let mask = 1 << pin;

        if (self.taken & mask) != 0 {
            loog::panic!("Pin {pin} is already in use");
        }

        self.taken |= mask;

        // SAFETY: this has been verified to be unique
        unsafe { AnyPin::steal(pin) }
    }
}
