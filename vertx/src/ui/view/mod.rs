mod about;
mod menu;

use embedded_graphics::prelude::*;

pub(super) use self::about::About;
pub(super) use self::menu::Menu;

pub(super) trait View: super::Component + Drawable<Output = ()> {
    fn title(&self) -> &str;
    fn input(&mut self, input: super::Input) -> super::StateChange;
}
