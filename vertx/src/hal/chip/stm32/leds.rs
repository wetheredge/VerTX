//! Based on <https://github.com/embassy-rs/embassy/blob/cae954a87ec3c5ece520b6a44168b36e79f3f86a/examples/stm32f4/src/bin/ws2812_pwm.rs>

use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::{Peripheral, gpio, timer};

pub(super) struct StatusDriver<'d, T: timer::GeneralInstance4Channel, D> {
    pwm: SimplePwm<'d, T>,
    dma: D,
    bit_0: u16,
    bit_1: u16,
}

impl<'d, T, D, DP> StatusDriver<'d, T, D>
where
    T: timer::GeneralInstance4Channel,
    D: Peripheral<P = DP>,
    DP: timer::UpDma<T>,
{
    pub(super) fn new(
        tim: impl Peripheral<P = T> + 'd,
        dma: D,
        pin: impl Peripheral<P = impl timer::Channel1Pin<T>> + 'd,
    ) -> Self {
        let pin = PwmPin::new_ch1(pin, gpio::OutputType::PushPull);
        let mut pwm = SimplePwm::new(
            tim,
            Some(pin),
            None,
            None,
            None,
            Hertz::khz(800),
            CountingMode::EdgeAlignedUp,
        );

        // Start low
        pwm.ch1().set_duty_cycle_fully_off();

        let max_duty = pwm.max_duty_cycle();
        let bit_0 = 8 * max_duty / 25;
        let bit_1 = 2 * bit_0;

        Self {
            pwm,
            dma,
            bit_0,
            bit_1,
        }
    }

    fn set_channel(&self, data: &mut [u16; 25], offset: u8, mut channel: u8) {
        for i in 0..8 {
            let bit = channel & 0x80;
            let bit = if bit == 0 { self.bit_0 } else { self.bit_1 };
            data[usize::from(offset + i)] = bit;
            channel = channel.overflowing_shl(1).0;
        }
    }
}

impl<T, D, DP> crate::hal::traits::StatusLed for StatusDriver<'_, T, D>
where
    T: timer::GeneralInstance4Channel,
    D: Peripheral<P = DP>,
    DP: timer::UpDma<T>,
{
    type Error = core::convert::Infallible;

    async fn set(&mut self, red: u8, green: u8, blue: u8) -> Result<(), Self::Error> {
        let mut data = [0; 25]; // 1 per bit per channel + trailing 0 to keep output low
        self.set_channel(&mut data, 0, red);
        self.set_channel(&mut data, 8, green);
        self.set_channel(&mut data, 16, blue);

        self.pwm
            .waveform_up(&mut self.dma, timer::Channel::Ch1, &data)
            .await;

        Ok(())
    }
}
