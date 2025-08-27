use display_interface::DisplayError;
use embassy_futures::select;
use embassy_time::Duration;
use embedded_graphics as eg;
use esp_hal::gpio;
use esp_hal::i2c::master::I2c;

use crate::hal;
use crate::ui::Input;

pub(super) struct Ui {
    pub(super) display: hal::display::Driver<I2c<'static, esp_hal::Async>>,
    pub(super) up: gpio::Input<'static>,
    pub(super) down: gpio::Input<'static>,
    pub(super) right: gpio::Input<'static>,
    pub(super) left: gpio::Input<'static>,
}

impl eg::geometry::OriginDimensions for Ui {
    fn size(&self) -> eg::geometry::Size {
        hal::display::SIZE
    }
}

impl eg::draw_target::DrawTarget for Ui {
    type Color = eg::pixelcolor::BinaryColor;
    type Error = DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = eg::Pixel<Self::Color>>,
    {
        self.display.draw_iter(pixels)
    }
}

impl hal::traits::Ui for Ui {
    async fn init(&mut self) -> Result<(), Self::Error> {
        hal::display::init(&mut self.display).await
    }

    async fn get_input(&mut self) -> Input {
        async fn debounced(pin: &mut gpio::Input<'static>, input: Input) -> Input {
            crate::utils::debounced_falling_edge(pin, Duration::from_millis(20)).await;
            input
        }

        let up = debounced(&mut self.up, Input::Up);
        let down = debounced(&mut self.down, Input::Down);
        let right = debounced(&mut self.right, Input::Forward);
        let left = debounced(&mut self.left, Input::Back);

        select::select_array([up, down, left, right]).await.0
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.display.flush().await
    }
}
