use alloc::borrow::Cow;

use embedded_graphics::geometry::AnchorX;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};

use super::View;
use crate::ui::component::{Component, List, ListItem};
use crate::ui::{Input, NextState, StateChange};

#[derive(Debug)]
pub(in crate::ui) struct Menu {
    current: Category,
    categories: List<Category>,
    submenu: List<NextState>,
    submenu_focused: bool,

    center: i32,
    height: i32,
    y_offset: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Category {
    #[default]
    Tools,
    Models,
}

static CATEGORIES: &[ListItem<Category>] = &[
    ListItem::new_const("Tools", Category::Tools),
    ListItem::new_const("Models", Category::Models),
];

static TOOLS: &[ListItem<NextState>] = &[
    ListItem::new_const("Configure", NextState::Configurator),
    ListItem::new_const("ELRS", NextState::ElrsConfig),
    ListItem::new_const("About", NextState::About),
];

static MODELS: &[ListItem<NextState>] = &[
    ListItem::new_const("Model No1", NextState::Model(0)),
    ListItem::new_const("Model No2", NextState::Model(1)),
    ListItem::new_const("Model No3", NextState::Model(2)),
    ListItem::new_const("Model No4", NextState::Model(3)),
    ListItem::new_const("Model No5", NextState::Model(4)),
    ListItem::new_const("Model No6", NextState::Model(5)),
    ListItem::new_const("Model No7", NextState::Model(6)),
    ListItem::new_const("Model No8", NextState::Model(7)),
    ListItem::new_const("Model No9", NextState::Model(8)),
];

impl Menu {
    pub(in crate::ui) fn new(bounds: Rectangle) -> Self {
        let center = bounds.resized_width(1, AnchorX::Center).top_left.x;

        let total_width = bounds.size.width;
        let height = bounds.size.height;
        let y_offset = bounds.top_left.y;

        let category_bounds = Rectangle::new(bounds.top_left, Size::new(center as u32 - 1, height));
        let submenu_bounds = Rectangle::new(
            Point::new(center + 2, y_offset),
            Size::new(total_width - center as u32 - 2, height),
        );

        let mut submenu = List::new(Cow::Borrowed(TOOLS), submenu_bounds);
        submenu.set_selection_visible(false);

        Self {
            current: Category::Tools,
            categories: List::new(Cow::Borrowed(CATEGORIES), category_bounds),
            submenu,
            submenu_focused: false,

            center,
            height: height as i32,
            y_offset,
        }
    }
}

impl Component for Menu {
    fn init<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let divider = Line::with_delta(
            Point::new(self.center, self.y_offset),
            Point::new(0, self.height),
        )
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1));
        divider.draw(target)?;

        self.categories.init(target)?;
        self.submenu.init(target)
    }
}

impl View for Menu {
    fn title(&self) -> &'static str {
        "Menu"
    }

    fn input(&mut self, input: Input) -> StateChange {
        if self.submenu_focused {
            match input {
                Input::Up => {
                    self.submenu.select_up();
                    StateChange::Update
                }
                Input::Down => {
                    self.submenu.select_down();
                    StateChange::Update
                }
                Input::Forward => match self.submenu.selected() {
                    Some(next) => StateChange::Push(*next),
                    None => StateChange::None,
                },
                Input::Back => {
                    self.submenu_focused = false;
                    self.submenu.set_selection_visible(false);
                    StateChange::Update
                }
            }
        } else {
            match input {
                Input::Up => {
                    self.categories.select_up();
                    StateChange::Update
                }
                Input::Down => {
                    self.categories.select_down();
                    StateChange::Update
                }
                Input::Forward => {
                    let Some(&submenu) = self.categories.selected() else {
                        // Should be unreachable
                        return StateChange::None;
                    };

                    if submenu != self.current {
                        let items = match submenu {
                            Category::Tools => Cow::Borrowed(TOOLS),
                            Category::Models => Cow::Borrowed(MODELS),
                        };
                        self.submenu.set_items(items);
                        self.current = submenu;
                    }

                    self.submenu_focused = true;
                    self.submenu.set_selection_visible(true);
                    StateChange::Update
                }
                Input::Back => StateChange::None,
            }
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
        self.categories.draw(target)?;
        self.submenu.draw(target)
    }
}
