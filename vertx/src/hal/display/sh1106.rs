use embedded_graphics as eg;
use embedded_hal_02::blocking::spi::{Transfer, Write};
use embedded_hal_02::digital::v2::OutputPin;
use sh1106::interface::{DisplayInterface, SpiInterface};
use sh1106::prelude::*;

pub(crate) const SIZE: eg::geometry::Size = eg::geometry::Size {
    width: 128,
    height: 64,
};

pub(crate) type Driver<S, P> = GraphicsMode<SpiInterface<S, P, P>>;

pub(crate) fn new<S: Write<u8, Error = E> + Transfer<u8, Error = E>, P: OutputPin, E>(
    spi: S,
    cs: P,
    dc: P,
) -> Driver<S, P> {
    sh1106::Builder::new().connect_spi(spi, cs, dc).into()
}

pub(crate) async fn init<I: DisplayInterface>(
    display: &mut GraphicsMode<I>,
) -> Result<(), I::Error> {
    display.init()
}
