use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;

use super::View;
use crate::ui::component::Component;
use crate::ui::{Input, StateChange};

#[derive(Debug)]
pub(in crate::ui) struct Model;

impl Component for Model {}

impl View for Model {
    fn title(&self) -> &'static str {
        "<Model name>"
    }

    fn input(&mut self, input: Input) -> StateChange {
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
