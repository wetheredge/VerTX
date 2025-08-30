use embassy_rp::pio::{self, PioPin, StateMachine};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_rp::{Peri, dma};
use smart_leds::RGB8;

pub struct StatusDriver<'d, P: pio::Instance, const SM: usize>(PioWs2812<'d, P, SM, 1>);

impl<'d, P: pio::Instance, const SM: usize> StatusDriver<'d, P, SM> {
    pub fn new(
        pio: &mut pio::Common<'d, P>,
        sm: StateMachine<'d, P, SM>,
        dma: Peri<'d, impl dma::Channel>,
        pin: Peri<'d, impl PioPin>,
    ) -> Self {
        let program = PioWs2812Program::new(pio);
        Self(PioWs2812::new(pio, sm, dma, pin, &program))
    }
}

impl<P: pio::Instance, const SM: usize> crate::hal::traits::StatusLed for StatusDriver<'_, P, SM> {
    type Error = core::convert::Infallible;

    async fn set(&mut self, red: u8, green: u8, blue: u8) -> Result<(), Self::Error> {
        self.0.write(&[RGB8::new(red, green, blue)]).await;
        Ok(())
    }
}
