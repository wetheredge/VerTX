use alloc::borrow::Cow;
use core::cmp::Ordering;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Baseline, Text};
use embedded_mogeefont::MogeeTextStyle;

use super::View;
use crate::ui::component::{Component, Scrolling};
use crate::ui::{Input, NextState, StateChange, LINE_HEIGHT};

#[derive(Debug)]
pub(in crate::ui) struct Menu {
    name: Cow<'static, str>,
    items: Cow<'static, [Item]>,
    selected: usize,
    bounds: Rectangle,
    scrolling: Scrolling,
}

#[derive(Debug, Clone)]
pub(super) struct Item {
    pub(super) name: Cow<'static, str>,
    pub(super) action: Action,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Action {
    Button(NextState),
}

impl Menu {
    pub(super) fn new(
        name: impl Into<Cow<'static, str>>,
        items: impl Into<Cow<'static, [Item]>>,
        bounds: Rectangle,
    ) -> Self {
        let items = items.into();
        let scrolling = Scrolling::new(items.len() as u32 * LINE_HEIGHT, bounds);
        let bounds = scrolling.inner_bounds();
        Self {
            name: name.into(),
            items,
            selected: 0,
            bounds,
            scrolling,
        }
    }

    fn scroll(&mut self, old_selected: usize) {
        match self.selected.cmp(&old_selected) {
            Ordering::Less => self.scrolling.scroll_up(LINE_HEIGHT * self.selected as u32),
            Ordering::Greater => self
                .scrolling
                .scroll_down(LINE_HEIGHT * (self.selected as u32 + 1)),
            Ordering::Equal => {}
        }
    }
}

impl Item {
    pub(super) const fn button(name: &'static str, value: NextState) -> Self {
        Self {
            name: Cow::Borrowed(name),
            action: Action::Button(value),
        }
    }
}

impl Component for Menu {
    fn init<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.scrolling.init(target)
    }
}

impl View for Menu {
    fn title(&self) -> &str {
        &self.name
    }

    fn input(&mut self, input: Input) -> StateChange {
        match input {
            Input::Up => {
                let old = self.selected;

                let selected = if self.selected == 0 {
                    self.items.len()
                } else {
                    self.selected
                };
                self.selected = selected - 1;

                self.scroll(old);
                StateChange::Update
            }
            Input::Down => {
                let old = self.selected;
                self.selected += 1;
                self.selected %= self.items.len();
                self.scroll(old);
                StateChange::Update
            }
            Input::Forward => match self.items[self.selected].action {
                Action::Button(next) => StateChange::Push(next),
            },
            Input::Back => StateChange::Pop,
        }
    }
}

impl Drawable for Menu {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let scroll_offset = self.scrolling.draw(target)?;

        // Crop to move origin point, then clip to ignore draws outside `self.bounds`
        let mut target = target.cropped(&self.bounds);
        let mut target = target.clipped(&target.bounding_box());
        target.clear(BinaryColor::Off)?;

        let style = MogeeTextStyle::new(BinaryColor::On);
        let style_selected = MogeeTextStyle::new(BinaryColor::Off);

        // TODO: use div_floor
        let skip = (scroll_offset / LINE_HEIGHT) as usize;
        let count = self.bounds.size.height.div_ceil(LINE_HEIGHT) as usize;
        for (i, item) in self.items.iter().enumerate().skip(skip).take(count) {
            let y = i as i32 * LINE_HEIGHT as i32 - scroll_offset as i32;

            let style = if i == self.selected {
                let width = target.bounding_box().size.width;
                target.fill_solid(
                    &Rectangle::new(Point::new(0, y), Size::new(width, LINE_HEIGHT)),
                    BinaryColor::On,
                )?;

                style_selected
            } else {
                style
            };

            // Offset to make initial verticals on the inverted selected entry clearer
            let position = Point::new(1, y);
            Text::with_baseline(&item.name, position, style, Baseline::Top).draw(&mut target)?;
        }

        Ok(())
    }
}
