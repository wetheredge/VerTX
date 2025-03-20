#![expect(unused_must_use)]

mod component;
mod history;
mod splash;
mod view;

use embassy_executor::task;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, StyledDrawable as _, Triangle};
use embedded_graphics::text::renderer::TextRenderer as _;
use embedded_graphics::text::{Baseline, Text};
use embedded_mogeefont::MogeeTextStyle;

use self::component::Component;
use self::history::History;
use self::view::View as _;
use crate::hal::traits::Ui as _;

type DrawResult<T = ()> = Result<T, <crate::hal::Ui as DrawTarget>::Error>;

const LINE_HEIGHT: u32 = 11;
const TITLE_HEIGHT: u32 = LINE_HEIGHT + 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Input {
    Up,
    Down,
    Forward,
    Back,
}

#[task]
pub(crate) async fn run(
    init: &'static crate::InitCounter,
    _config: crate::Config,
    mut ui: crate::hal::Ui,
    models: crate::models::Manager,
    #[cfg(feature = "configurator")] configurator: crate::configurator::Manager,
) -> ! {
    let init = init.start(loog::intern!("ui"));

    loog::debug_assert_eq!(
        LINE_HEIGHT,
        MogeeTextStyle::new(BinaryColor::On).line_height(),
        "LINE_HEIGHT is incorrect"
    );

    let ui = &mut ui;

    ui.init().await;
    splash::run(ui).await;

    let below_title = {
        let mut bounds = ui.bounding_box();
        bounds.top_left.y += TITLE_HEIGHT as i32;
        bounds.size.height -= TITLE_HEIGHT;
        bounds
    };

    let mut menu = State::Menu(view::Menu::new(below_title, models));
    menu.init(true, ui);
    ui.flush().await;

    init.finish();

    let mut stack = History::<3>::new(menu);
    loop {
        let current = stack.current();

        let input = ui.get_input().await;
        match current.input(input).await {
            StateChange::None => {}
            StateChange::Update => {
                current.draw(ui);
                ui.flush().await;
            }
            StateChange::Push(next) => {
                let next = match next {
                    NextState::Model(raw_name) => match models.open(raw_name).await {
                        Ok(model) => Some(State::Model(view::Model::new(model))),
                        Err(err) => {
                            loog::error!("Failed to open model: {err:?}");
                            None
                        }
                    },
                    #[cfg(feature = "configurator")]
                    NextState::Configurator => {
                        configurator.start();
                        None
                    }
                    NextState::ElrsConfig => {
                        loog::warn!("TODO: ELRS config tool");
                        None
                    }
                    NextState::About => Some(State::About(view::About::new(below_title))),
                };

                if let Some(mut next) = next {
                    // Can't be root bc this will be pushed onto the top of the stack
                    next.init(false, ui);
                    ui.flush().await;
                    stack.push(next);
                }
            }
            StateChange::Pop => {
                stack.pop();
                let is_root = stack.is_root();
                stack.current().init(is_root, ui);
                ui.flush().await;
            }
        }
    }
}

#[derive(Debug)]
enum State {
    Model(view::Model),
    Menu(view::Menu),
    #[expect(dead_code)]
    #[cfg(feature = "network")]
    Wifi {
        network: Option<WifiNetwork>,
        uri: heapless::String<32>,
    },
    #[expect(dead_code)]
    ElrsConfig,
    About(view::About),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NextState {
    Model(crate::models::RawName),
    #[cfg(feature = "configurator")]
    Configurator,
    ElrsConfig,
    About,
}

#[expect(dead_code)]
#[derive(Debug, Clone)]
#[cfg(feature = "network")]
struct WifiNetwork {
    show: bool,
    ssid: heapless::String<16>,
    password: heapless::String<16>,
}

#[cfg(feature = "network")]
impl WifiNetwork {
    #[expect(dead_code)]
    fn new(ssid: &crate::network::Ssid, password: &crate::network::Password) -> Option<Self> {
        if let Ok(ssid) = ssid.as_str().try_into() {
            if let Ok(password) = password.as_str().try_into() {
                return Some(Self {
                    show: true,
                    ssid,
                    password,
                });
            }
        }

        None
    }
}

enum StateChange {
    None,
    Update,
    Push(NextState),
    Pop,
}

impl State {
    fn init(&mut self, is_root: bool, display: &mut crate::hal::Ui) -> DrawResult {
        display.clear(BinaryColor::Off)?;

        let title = match self {
            Self::Model(model) => {
                model.init(display)?;
                model.title()
            }
            Self::Menu(menu) => {
                menu.init(display)?;
                menu.title()
            }
            #[cfg(feature = "network")]
            Self::Wifi { .. } => todo!(),
            Self::ElrsConfig => todo!(),
            Self::About(about) => {
                about.init(display)?;
                about.title()
            }
        };

        draw_title(is_root, title, display)?;
        self.draw(display)
    }

    async fn input(&mut self, input: Input) -> StateChange {
        match self {
            State::Model(model) => model.input(input).await,
            State::Menu(menu) => menu.input(input).await,
            #[cfg(feature = "network")]
            State::Wifi { .. } => todo!(),
            State::ElrsConfig => todo!(),
            State::About(about) => about.input(input).await,
        }
    }
}

impl Drawable for State {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        match self {
            Self::Model(model) => model.draw(target),
            Self::Menu(menu) => menu.draw(target),
            #[cfg(feature = "network")]
            Self::Wifi { .. } => todo!(),
            Self::ElrsConfig => todo!(),
            Self::About(about) => about.draw(target),
        }
    }
}

fn draw_title(is_root: bool, title: &str, display: &mut crate::hal::Ui) -> DrawResult {
    let x = if is_root {
        1
    } else {
        // Bias toward bottom when odd
        let y_center = (LINE_HEIGHT / 2) as i32;
        let y_delta = 2;
        let width = 2;

        let v1 = Point::new(0, y_center);
        let v2 = Point::new(width, y_center - y_delta);
        let v3 = Point::new(width, y_center + y_delta);
        let arrow = Triangle::new(v1, v2, v3);
        arrow.draw_styled(&PrimitiveStyle::with_fill(BinaryColor::On), display)?;

        width + 3
    };

    let y = TITLE_HEIGHT as i32 - 2;
    let width = display.bounding_box().size.width;
    display.fill_solid(
        &Rectangle::new(Point::new(0, y), Size::new(width, 1)),
        BinaryColor::On,
    )?;

    let text_style = MogeeTextStyle::new(BinaryColor::On);
    Text::with_baseline(title, Point::new(x, 0), text_style, Baseline::Top).draw(display)?;

    Ok(())
}
