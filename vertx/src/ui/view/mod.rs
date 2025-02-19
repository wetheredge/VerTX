mod about;
mod menu;

use alloc::vec;

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

pub(super) use self::about::About;
pub(super) use self::menu::Menu;
use super::NextState;
use crate::ui::component::ListItem;

pub(super) trait View: super::Component + Drawable<Output = ()> {
    fn title(&self) -> &str;
    fn input(&mut self, input: super::Input) -> super::StateChange;
}

pub(super) fn main_menu(bounds: Rectangle) -> Menu {
    let items = vec![
        ListItem::text("Models", NextState::ModelSelect),
        ListItem::text("Configure", NextState::Configurator),
        ListItem::text("ELRS", NextState::ElrsConfig),
        ListItem::text("About", NextState::About),
    ];
    Menu::new("Menu", items, bounds)
}

pub(super) fn model_menu(bounds: Rectangle) -> Menu {
    let items = vec![ListItem::text("Main menu", NextState::MainMenu)];
    Menu::new("Models", items, bounds)
}
