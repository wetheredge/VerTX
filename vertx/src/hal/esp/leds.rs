//! WS2812 driver using the RMT
//!
//! Based on <https://github.com/esp-rs/esp-hal-community/blob/f78ff85fcb485f11b2b2e3a88b13d1c5488af958/esp-hal-smartled/src/lib.rs>.

use core::slice::IterMut;

use esp_hal::clock::Clocks;
use esp_hal::gpio::OutputPin;
use esp_hal::peripheral::Peripheral;
use esp_hal::rmt;

const PERIOD: u32 = 1250; // 800kHz
const T0H_NS: u32 = 400; // 300ns per SK6812 datasheet, 400 per WS2812. Some require >350ns for T0H. Others <500ns for T0H.
const T0L_NS: u32 = PERIOD - T0H_NS;
const T1H_NS: u32 = 850; // 900ns per SK6812 datasheet, 850 per WS2812. > 550ns is sometimes enough. Some require T1H >= 2 * T0H. Some require > 300ns T1L.
const T1L_NS: u32 = PERIOD - T1H_NS;

pub struct StatusLed<Tx> {
    channel: Option<Tx>,
    pulses: (u32, u32),
}

impl<Tx: rmt::TxChannel> StatusLed<Tx> {
    pub fn new<'d, C, P>(channel: C, pin: impl Peripheral<P = P> + 'd) -> Self
    where
        C: rmt::TxChannelCreator<'d, Tx, P>,
        P: OutputPin + 'd,
    {
        let config = rmt::TxChannelConfig {
            clk_divider: 1,
            idle_output_level: false,
            carrier_modulation: false,
            idle_output: true,

            ..Default::default()
        };

        let channel = channel.configure(pin, config).unwrap();

        // Assume the RMT peripheral is set up to use the APB clock
        let clocks = Clocks::get();
        let src_clock = clocks.apb_clock.to_MHz();

        Self {
            channel: Some(channel),
            pulses: (
                u32::from(rmt::PulseCode {
                    level1: true,
                    length1: ((T0H_NS * src_clock) / 1000) as u16,
                    level2: false,
                    length2: ((T0L_NS * src_clock) / 1000) as u16,
                }),
                u32::from(rmt::PulseCode {
                    level1: true,
                    length1: ((T1H_NS * src_clock) / 1000) as u16,
                    level2: false,
                    length2: ((T1L_NS * src_clock) / 1000) as u16,
                }),
            ),
        }
    }
}

impl<Tx: rmt::TxChannel> crate::hal::traits::StatusLed for StatusLed<Tx> {
    type Error = rmt::Error;

    async fn set(&mut self, red: u8, green: u8, blue: u8) -> Result<(), Self::Error> {
        // 3 channels * 8 bits/pulses + stop
        let mut buffer = [0u32; 25];

        let buffer_iter = &mut buffer.iter_mut();
        push_pulses(green, buffer_iter, self.pulses);
        push_pulses(red, buffer_iter, self.pulses);
        push_pulses(blue, buffer_iter, self.pulses);

        // Perform the actual RMT operation
        let channel = self.channel.take().unwrap();
        let (channel, result) = match channel.transmit(&buffer).wait() {
            Ok(channel) => (channel, Ok(())),
            Err((err, channel)) => (channel, Err(err)),
        };
        self.channel = Some(channel);
        result
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
