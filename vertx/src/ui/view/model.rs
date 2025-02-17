use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;

use super::View;
use crate::models;
use crate::ui::component::Component;
use crate::ui::{Input, StateChange};

#[derive(Debug)]
pub(in crate::ui) struct Model {
    model: models::Model,
}

impl Model {
    pub(in crate::ui) fn new(model: models::Model) -> Self {
        Self { model }
    }
}

impl Component for Model {}

impl View for Model {
    fn title(&self) -> &str {
        self.model.name()
    }

    async fn input(&mut self, input: Input) -> StateChange {
        if input == Input::Back {
            StateChange::Pop
        } else {
            StateChange::None
        }
    }
}

impl Drawable for Model {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, _target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        Ok(())
    }
}
