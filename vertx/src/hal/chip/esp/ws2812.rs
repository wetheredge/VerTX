//! WS2812 driver using the RMT
//!
//! Based on <https://github.com/esp-rs/esp-hal-community/blob/f78ff85fcb485f11b2b2e3a88b13d1c5488af958/esp-hal-smartled/src/lib.rs>.

use core::slice::IterMut;

use esp_hal::clock::Clocks;
use esp_hal::gpio::{self, OutputPin};
use esp_hal::peripheral::Peripheral;
use esp_hal::rmt;

const PERIOD: u32 = 1250; // 800kHz
const T0H_NS: u32 = 400; // 300ns per SK6812 datasheet, 400 per WS2812. Some require >350ns for T0H. Others <500ns for T0H.
const T0L_NS: u32 = PERIOD - T0H_NS;
const T1H_NS: u32 = 850; // 900ns per SK6812 datasheet, 850 per WS2812. > 550ns is sometimes enough. Some require T1H >= 2 * T0H. Some require > 300ns T1L.
const T1L_NS: u32 = PERIOD - T1H_NS;

pub struct StatusLed<Tx> {
    channel: Tx,
    pulses: (u32, u32),
}

impl<Tx: rmt::TxChannelAsync> StatusLed<Tx> {
    pub fn new<'d, C, P>(channel: C, pin: impl Peripheral<P = P> + 'd) -> Self
    where
        C: rmt::TxChannelCreatorAsync<'d, Tx, P>,
        P: OutputPin + 'd,
    {
        let config = rmt::TxChannelConfig::default()
            .with_clk_divider(1)
            .with_carrier_modulation(false)
            .with_idle_output(true)
            .with_idle_output_level(gpio::Level::Low);

        let channel = channel.configure(pin, config).unwrap();

        // Assume the RMT peripheral is set up to use the APB clock
        let clocks = Clocks::get();
        let src_clock = clocks.apb_clock.as_mhz();

        Self {
            channel,
            pulses: (
                rmt::PulseCode::new(
                    gpio::Level::High,
                    ((T0H_NS * src_clock) / 1000) as u16,
                    gpio::Level::Low,
                    ((T0L_NS * src_clock) / 1000) as u16,
                ),
                rmt::PulseCode::new(
                    gpio::Level::High,
                    ((T1H_NS * src_clock) / 1000) as u16,
                    gpio::Level::Low,
                    ((T1L_NS * src_clock) / 1000) as u16,
                ),
            ),
        }
    }
}

impl<Tx: rmt::TxChannelAsync> crate::hal::traits::StatusLed for StatusLed<Tx> {
    type Error = rmt::Error;

    async fn set(&mut self, color: crate::leds::Color) -> Result<(), Self::Error> {
        // 3 channels * 8 bits/pulses + stop
        let mut buffer = [rmt::PulseCode::empty(); 25];

        let buffer_iter = &mut buffer.iter_mut();
        push_pulses(color.g, buffer_iter, self.pulses);
        push_pulses(color.r, buffer_iter, self.pulses);
        push_pulses(color.b, buffer_iter, self.pulses);

        self.channel.transmit(&buffer).await
    }
}

fn push_pulses(value: u8, out: &mut IterMut<u32>, pulses: (u32, u32)) {
    for position in [128, 64, 32, 16, 8, 4, 2, 1] {
        *out.next().unwrap() = match value & position {
            0 => pulses.0,
            _ => pulses.1,
        }
    }
}
