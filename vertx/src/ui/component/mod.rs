mod list;
mod qr_code;
mod scrolling;

use embedded_graphics::prelude::*;

pub(super) use self::list::{Item as ListItem, List};
pub(super) use self::qr_code::QrCode;
pub(super) use self::scrolling::Scrolling;

pub(super) trait Component: Drawable {
    fn init<D>(&self, _target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        Ok(())
    }
}
