use embedded_graphics as eg;
use embedded_hal_async::i2c::I2c;
use ssd1306::prelude::*;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};

pub(crate) const SIZE: eg::geometry::Size = eg::geometry::Size {
    width: 128,
    height: 64,
};

type Size = DisplaySize128x64;
pub(crate) type Driver<I> =
    Ssd1306Async<I2CInterface<I>, Size, ssd1306::mode::BufferedGraphicsModeAsync<Size>>;

pub(crate) fn new<I: I2c>(i2c: I) -> Driver<I> {
    let interface = I2CDisplayInterface::new(i2c);
    Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode()
}

pub(crate) async fn init<D: DisplayConfigAsync>(display: &mut D) -> Result<(), D::Error> {
    display.init().await
}
