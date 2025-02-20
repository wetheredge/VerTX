use alloc::borrow::Cow;
use core::cmp::Ordering;

use embedded_graphics::geometry::Point;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use embedded_mogeefont::MogeeTextStyle;

use super::{Component, Scrolling};
use crate::ui::LINE_HEIGHT;

#[derive(Debug)]
pub(in crate::ui) struct List<A: Clone + 'static> {
    items: Cow<'static, [Item<A>]>,
    selected: usize,
    selection_visible: bool,
    scrolling: Scrolling,
}

#[derive(Debug, Clone)]
pub(in crate::ui) enum Item<A> {
    Text { label: Cow<'static, str>, action: A },
    Divider,
}

impl<A: Clone + 'static> List<A> {
    pub(in crate::ui) fn new(items: Cow<'static, [Item<A>]>, bounds: Rectangle) -> Self {
        let scrolling = Scrolling::new(0, bounds);

        let mut list = Self {
            items: Cow::Borrowed(&[]),
            selected: 0,
            selection_visible: true,
            scrolling,
        };

        list.set_items(items);
        list
    }

    pub(in crate::ui) fn set_items(&mut self, items: Cow<'static, [Item<A>]>) {
        self.selected = items
            .iter()
            .position(Item::is_selectable)
            .unwrap_or_default();
        self.scrolling.set_height(items.len() as u32 * LINE_HEIGHT);
        self.items = items;
    }

    pub(in crate::ui) fn set_selection_visible(&mut self, visible: bool) {
        self.selection_visible = visible;
    }

    /// Get the action associated with the currently selected item. Returns
    /// `None` if there aren't any selectable items.
    pub(in crate::ui) fn selected(&self) -> Option<&A> {
        match self.items.get(self.selected) {
            Some(Item::Text { action, .. }) => Some(action),
            Some(Item::Divider) | None => None,
        }
    }

    pub(in crate::ui) fn select_up(&mut self) {
        if self.items.len() <= 1 {
            // No point in scrolling
            return;
        }

        let mut next = self.selected;
        loop {
            if next == 0 {
                next = self.items.len();
            }
            next -= 1;

            if next == self.selected {
                // Wrapped around to the same entry, so cancel scroll whether or not this is
                // actually selectable
                return;
            }

            if self.items[next].is_selectable() {
                break;
            }
        }

        self.scroll(next);
    }

    pub(in crate::ui) fn select_down(&mut self) {
        if self.items.len() <= 1 {
            // No point in scrolling
            return;
        }

        let mut next = self.selected;
        loop {
            next = (next + 1) % self.items.len();

            if next == self.selected {
                // Wrapped around to the same entry, so cancel scroll whether or not this is
                // actually selectable
                return;
            }

            if self.items[next].is_selectable() {
                break;
            }
        }

        self.scroll(next);
    }

    fn scroll(&mut self, to: usize) {
        let from = self.selected;
        self.selected = to;

        // TODO: keep non-selectable items on screen if possible when wrapping around?
        match self.selected.cmp(&from) {
            Ordering::Less => self.scrolling.scroll_up(LINE_HEIGHT * self.selected as u32),
            Ordering::Greater => self
                .scrolling
                .scroll_down(LINE_HEIGHT * (self.selected as u32 + 1)),
            Ordering::Equal => {}
        }
    }
}

impl<A: Clone + 'static> Item<A> {
    pub(in crate::ui) const fn const_text(label: &'static str, action: A) -> Self {
        Self::Text {
            label: Cow::Borrowed(label),
            action,
        }
    }

    fn is_selectable(&self) -> bool {
        match self {
            Item::Text { .. } => true,
            Item::Divider => false,
        }
    }
}

impl<A: Clone + 'static> Component for List<A> {
    fn init<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.scrolling.init(target)
    }
}

impl<A: Clone + 'static> Drawable for List<A> {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let scroll_offset = self.scrolling.draw(target)?;
        let bounds = self.scrolling.inner_bounds();

        // Crop to move origin point, then clip to ignore out-of-bounds draws
        let mut target = target.cropped(&bounds);
        let mut target = target.clipped(&target.bounding_box());
        target.clear(BinaryColor::Off)?;

        let style = MogeeTextStyle::new(BinaryColor::On);
        let style_selected = MogeeTextStyle::new(BinaryColor::Off);

        // TODO: use div_floor
        let skip = (scroll_offset / LINE_HEIGHT) as usize;
        let count = bounds.size.height.div_ceil(LINE_HEIGHT) as usize;
        for (i, item) in self.items.iter().enumerate().skip(skip).take(count) {
            let y = i as i32 * LINE_HEIGHT as i32 - scroll_offset as i32;

            match item {
                Item::Text { label, .. } => {
                    let style = if i == self.selected && self.selection_visible {
                        let width = target.bounding_box().size.width;
                        target.fill_solid(
                            &Rectangle::new(Point::new(0, y), Size::new(width, LINE_HEIGHT)),
                            BinaryColor::On,
                        )?;

                        style_selected
                    } else {
                        style
                    };

                    // Offset to make initial verticals on the inverted item clearer
                    let position = Point::new(1, y);

                    let text = Text::with_baseline(label, position, style, Baseline::Top);
                    text.draw(&mut target)?;
                }
                Item::Divider => {
                    let y = y + LINE_HEIGHT as i32 / 2;
                    let thickness = (LINE_HEIGHT % 2) + 1;

                    let gap = thickness;
                    let length = 8;
                    let mut x = gap as i32;
                    for _ in 0..3 {
                        let line = Line::new(Point::new(x, y), Point::new(x + length - 1, y))
                            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, thickness));
                        line.draw(&mut target)?;
                        x += length + gap as i32;
                    }
                }
            }
        }

        Ok(())
    }
}
