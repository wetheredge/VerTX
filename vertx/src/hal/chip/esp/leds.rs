//! WS2812 driver using the RMT
//!
//! Based on <https://github.com/esp-rs/esp-hal-community/blob/f78ff85fcb485f11b2b2e3a88b13d1c5488af958/esp-hal-smartled/src/lib.rs>.

use core::slice::IterMut;

use esp_hal::clock::Clocks;
use esp_hal::gpio::{self, OutputPin};
use esp_hal::rmt;
use esp_hal::rmt::TxChannelAsync as _;

const PERIOD: u32 = 1250; // 800kHz
const T0H_NS: u32 = 400; // 300ns per SK6812 datasheet, 400 per WS2812. Some require >350ns for T0H. Others <500ns for T0H.
const T0L_NS: u32 = PERIOD - T0H_NS;
const T1H_NS: u32 = 850; // 900ns per SK6812 datasheet, 850 per WS2812. > 550ns is sometimes enough. Some require T1H >= 2 * T0H. Some require > 300ns T1L.
const T1L_NS: u32 = PERIOD - T1H_NS;

pub struct StatusLed {
    channel: rmt::AnyTxChannel<esp_hal::Async>,
    pulses: (u32, u32),
}

impl StatusLed {
    pub fn new<'d, C, R>(channel: C, pin: impl OutputPin + 'd) -> Self
    where
        C: rmt::TxChannelCreator<'d, esp_hal::Async, Raw = R>,
        R: rmt::RawChannelAccess<Dir = rmt::Tx>,
    {
        let config = rmt::TxChannelConfig::default()
            .with_clk_divider(1)
            .with_carrier_modulation(false)
            .with_idle_output(true)
            .with_idle_output_level(gpio::Level::Low);

        let channel = channel.configure_tx(pin, config).unwrap();

        // Assume the RMT peripheral is set up to use the APB clock
        let clocks = Clocks::get();
        let src_clock = clocks.apb_clock.as_mhz();

        Self {
            channel: channel.degrade(),
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

impl crate::hal::traits::StatusLed for StatusLed {
    type Error = rmt::Error;

    async fn set(&mut self, red: u8, green: u8, blue: u8) -> Result<(), Self::Error> {
        // 3 channels * 8 bits/pulses + stop
        let mut buffer = [rmt::PulseCode::empty(); 25];

        let buffer_iter = &mut buffer.iter_mut();
        push_pulses(green, buffer_iter, self.pulses);
        push_pulses(red, buffer_iter, self.pulses);
        push_pulses(blue, buffer_iter, self.pulses);

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
