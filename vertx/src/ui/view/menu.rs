use alloc::borrow::Cow;
use alloc::vec::Vec;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use super::View;
use crate::ui::component::{Component, List, ListItem};
use crate::ui::{Input, NextState, StateChange};

#[derive(Debug)]
pub(in crate::ui) struct Menu {
    name: Cow<'static, str>,
    list: List<NextState>,
}

impl Menu {
    pub(super) fn new(
        name: impl Into<Cow<'static, str>>,
        items: Vec<ListItem<NextState>>,
        bounds: Rectangle,
    ) -> Self {
        Self {
            name: name.into(),
            list: List::new(items, bounds),
        }
    }
}

impl Component for Menu {
    fn init<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.list.init(target)
    }
}

impl View for Menu {
    fn title(&self) -> &str {
        &self.name
    }

    fn input(&mut self, input: Input) -> StateChange {
        match input {
            Input::Up => {
                self.list.select_up();
                StateChange::Update
            }
            Input::Down => {
                self.list.select_down();
                StateChange::Update
            }
            Input::Forward => match self.list.selected() {
                Some(next) => StateChange::Push(*next),
                None => StateChange::None,
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
        self.list.draw(target)
    }
}
