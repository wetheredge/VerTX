//! WS2812 driver using a PIO module
//!
//! Based on <https://github.com/embassy-rs/embassy/blob/ff3f354893a77f4aba0025c047568dd6855f3a4f/examples/rp/src/bin/pio_ws2812.rs>.

use embassy_rp::pio::{Instance as PioInstance, PioPin, StateMachine};
use embassy_rp::{Peri, clocks};
use embassy_time::Timer;
use fixed::types::U24F8;
use fixed_macro::types::U24F8;

pub(super) struct StatusDriver<'d, P: PioInstance, const SM: usize> {
    sm: StateMachine<'d, P, SM>,
}

impl<'d, P: PioInstance, const SM: usize> StatusDriver<'d, P, SM> {
    pub(super) fn new(
        pio: &mut embassy_rp::pio::Common<'d, P>,
        mut sm: StateMachine<'d, P, SM>,
        pin: Peri<'d, impl PioPin>,
    ) -> Self {
        // prepare the PIO program
        let side_set = pio::SideSet::new(false, 1, false);
        let mut a: pio::Assembler<32> = pio::Assembler::new_with_side_set(side_set);

        const T1: u8 = 2; // start bit
        const T2: u8 = 5; // data bit
        const T3: u8 = 3; // stop bit
        const CYCLES_PER_BIT: u32 = (T1 + T2 + T3) as u32;

        let mut wrap_target = a.label();
        let mut wrap_source = a.label();
        let mut do_zero = a.label();
        a.set_with_side_set(pio::SetDestination::PINDIRS, 1, 0);
        a.bind(&mut wrap_target);
        // Do stop bit
        a.out_with_delay_and_side_set(pio::OutDestination::X, 1, T3 - 1, 0);
        // Do start bit
        a.jmp_with_delay_and_side_set(pio::JmpCondition::XIsZero, &mut do_zero, T1 - 1, 1);
        // Do data bit = 1
        a.jmp_with_delay_and_side_set(pio::JmpCondition::Always, &mut wrap_target, T2 - 1, 1);
        a.bind(&mut do_zero);
        // Do data bit = 0
        a.nop_with_delay_and_side_set(T2 - 1, 0);
        a.bind(&mut wrap_source);

        let prg = a.assemble_with_wrap(wrap_source, wrap_target);
        let mut cfg = embassy_rp::pio::Config::default();

        // Pin config
        let out_pin = pio.make_pio_pin(pin);
        cfg.set_out_pins(&[&out_pin]);
        cfg.set_set_pins(&[&out_pin]);

        cfg.use_program(&pio.load_program(&prg), &[&out_pin]);

        // Clock config, measured in kHz to avoid overflows
        let clock_freq = U24F8::from_num(clocks::clk_sys_freq() / 1000);
        let ws2812_freq = U24F8!(800);
        let bit_freq = ws2812_freq * CYCLES_PER_BIT;
        cfg.clock_divider = clock_freq / bit_freq;

        // FIFO config
        cfg.fifo_join = embassy_rp::pio::FifoJoin::TxOnly;
        cfg.shift_out = embassy_rp::pio::ShiftConfig {
            auto_fill: true,
            threshold: 24,
            direction: embassy_rp::pio::ShiftDirection::Left,
        };

        sm.set_config(&cfg);
        sm.set_enable(true);

        Self { sm }
    }
}

impl<P: PioInstance, const SM: usize> crate::hal::traits::StatusLed for StatusDriver<'_, P, SM> {
    type Error = core::convert::Infallible;

    async fn set(&mut self, red: u8, green: u8, blue: u8) -> Result<(), Self::Error> {
        let word = (u32::from(green) << 24) | (u32::from(red) << 16) | (u32::from(blue) << 8);
        self.sm.tx().push(word);
        Timer::after_micros(55).await;
        Ok(())
    }
}
