mod about;
mod menu;
mod model;

use embedded_graphics::prelude::*;

pub(super) use self::about::About;
pub(super) use self::menu::Menu;
pub(super) use self::model::Model;

pub(super) trait View: super::Component + Drawable<Output = ()> {
    fn title(&self) -> &str;
    async fn input(&mut self, input: super::Input) -> super::StateChange;
}
