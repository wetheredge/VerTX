use embedded_graphics::geometry::AnchorX;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use super::Component;

const BAR_WIDTH: u32 = 1;
const BAR_MARGIN: u32 = 1;

#[derive(Debug, Clone)]
pub(in crate::ui) struct Scrolling {
    height: u32,
    bounds: Rectangle,
    offset: u32,

    bar_height: u32,
    bar_y: i32,
}

impl Scrolling {
    pub(in crate::ui) fn new(height: u32, bounds: Rectangle) -> Self {
        let visible_height = bounds.size.height;
        let bar_height = if visible_height < height {
            (visible_height * visible_height / height).max(1)
        } else {
            0
        };

        Self {
            height,
            bounds,
            offset: 0,

            bar_height,
            bar_y: 0,
        }
    }

    pub(in crate::ui) const fn bar_allowance() -> u32 {
        BAR_WIDTH + BAR_MARGIN
    }

    pub(in crate::ui) const fn inner_bounds(&self) -> Rectangle {
        let mut inner = self.bounds;
        if self.can_scroll() {
            inner.size.width = inner.size.width.saturating_sub(Self::bar_allowance());
        }
        inner
    }

    pub(in crate::ui) fn scroll_by(&mut self, offset: i32) {
        self.set_offset(self.offset.saturating_add_signed(offset));
    }

    pub(in crate::ui) fn scroll_up(&mut self, top: u32) {
        self.set_offset(top.min(self.offset));
    }

    pub(in crate::ui) fn scroll_down(&mut self, bottom: u32) {
        self.set_offset(
            bottom
                .saturating_sub(self.visible_height())
                .max(self.offset),
        );
    }

    const fn visible_height(&self) -> u32 {
        self.bounds.size.height
    }

    const fn can_scroll(&self) -> bool {
        self.visible_height() < self.height
    }

    fn set_offset(&mut self, offset: u32) {
        if self.offset == offset || !self.can_scroll() {
            return;
        }

        self.offset = offset.min(self.height - self.visible_height());

        let available_y = self.visible_height() - self.bar_height;
        let scroll_distance = self.height - self.visible_height();
        // (offset / scroll_distance) * available_y
        self.bar_y = div_round(available_y * self.offset, scroll_distance) as i32;
    }
}

impl Component for Scrolling {
    fn init<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        Ok(())
    }
}

impl Drawable for Scrolling {
    type Color = BinaryColor;
    type Output = u32;

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        if !self.can_scroll() {
            return Ok(0);
        }

        let mut bar = self.bounds.resized_width(BAR_WIDTH, AnchorX::Right);
        target.fill_solid(&bar, BinaryColor::Off)?;
        bar.top_left.y += self.bar_y;
        bar.size.height = self.bar_height;
        target.fill_solid(&bar, BinaryColor::On)?;

        Ok(self.offset)
    }
}

fn div_round(lhs: u32, rhs: u32) -> u32 {
    let quotient = lhs / rhs;
    if ((lhs % rhs) * 2) > rhs {
        quotient + 1
    } else {
        quotient
    }
}
