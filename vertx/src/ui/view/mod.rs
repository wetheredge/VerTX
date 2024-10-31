mod about;
mod menu;

use alloc::vec;

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

pub(super) use self::about::About;
pub(super) use self::menu::Menu;

pub(super) trait View: super::Component + Drawable<Output = ()> {
    fn title(&self) -> &str;
    fn input(&mut self, input: super::Input) -> super::StateChange;
}

pub(super) fn main_menu(bounds: Rectangle) -> Menu {
    static ITEMS: &[menu::Item] = &[
        menu::Item::button("Models", super::NextState::ModelSelect),
        menu::Item::button("Configure", super::NextState::Configurator),
        menu::Item::button("ELRS", super::NextState::ElrsConfig),
        menu::Item::button("About", super::NextState::About),
    ];
    Menu::new("Menu", ITEMS, bounds)
}

pub(super) fn model_menu(bounds: Rectangle) -> Menu {
    let items = vec![menu::Item::button("Main menu", super::NextState::MainMenu)];
    Menu::new("Models", items, bounds)
}
