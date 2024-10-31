use core::iter;

use embedded_graphics::geometry::AnchorPoint;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use qrcodegen_no_heap::{QrCodeEcc, Version};

use super::Component;

/// Pixels per module
const SCALE: u32 = 2;
/// For version 2
const SIZE: u32 = 25 * SCALE;

#[derive(Debug)]
pub(in crate::ui) struct QrCode<'a> {
    bounds: Rectangle,
    anchor: AnchorPoint,
    data: &'a str,
}

impl<'a> QrCode<'a> {
    pub(in crate::ui) fn new(
        data: &'a str,
        bounds: Rectangle,
        anchor: AnchorPoint,
    ) -> Option<Self> {
        let size = bounds.size;
        (size.height >= SIZE && size.width >= SIZE).then_some(Self {
            bounds,
            anchor,
            data,
        })
    }

    pub(in crate::ui) const fn size() -> u32 {
        SIZE
    }
}

impl Component for QrCode<'_> {
    fn init<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        const VERSION: Version = Version::new(2);

        let mut temp = [0; VERSION.buffer_len()];
        let mut out = [0; VERSION.buffer_len()];

        let qr_code = qrcodegen_no_heap::QrCode::encode_text(
            self.data,
            &mut temp,
            &mut out,
            QrCodeEcc::Low,
            VERSION,
            VERSION,
            None,
            true,
        );

        if let Ok(qr_code) = qr_code {
            loog::debug_assert_eq!(Self::size() / SCALE, qr_code.size() as u32);

            let bounds = self
                .bounds
                .resized(Size::new_equal(Self::size()), self.anchor);

            let mut i = 0;
            let pixels = iter::from_fn(|| {
                let x = i / Self::size();
                let y = i % Self::size();

                if y > Self::size() {
                    return None;
                }

                let qr_x = (x / SCALE) as i32;
                let qr_y = (y / SCALE) as i32;

                i += 1;
                Some(BinaryColor::from(qr_code.get_module(qr_x, qr_y)))
            });

            target.fill_contiguous(&bounds, pixels)
        } else {
            Ok(())
        }
    }
}

impl Drawable for QrCode<'_> {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        Ok(())
    }
}
