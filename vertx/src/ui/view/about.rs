use core::fmt;

use embedded_graphics::geometry::AnchorPoint;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_mogeefont::MogeeTextStyle;
use embedded_text::TextBox;
use embedded_text::plugin::NoPlugin;
use embedded_text::style::{HeightMode, TextBoxStyle};

use super::View;
use crate::build_info;
use crate::ui::component::{Component, QrCode, Scrolling};
use crate::ui::{Input, LINE_HEIGHT, StateChange};

const DEBUG: &str = if cfg!(debug_assertions) {
    "debug\n"
} else {
    ""
};
static ABOUT: &str = const_format::formatcp!(
    "{target}\n{DEBUG}v{version}\n{commit} ({branch})\n\n{home}",
    target = build_info::TARGET,
    version = build_info::VERSION,
    commit = build_info::GIT_COMMIT,
    branch = build_info::GIT_BRANCH,
    home = env!("CARGO_PKG_HOMEPAGE"),
);
static QR_URL: &str = include_str!(concat!(env!("OUT_DIR"), "/qr_url"));

pub(in crate::ui) struct About {
    text_bounds: Rectangle,
    text: TextBox<'static, MogeeTextStyle<BinaryColor>, NoPlugin<BinaryColor>>,
    qr_code: QrCode<'static>,
    scrolling: Scrolling,
}

impl About {
    pub(in crate::ui) fn new(bounds: Rectangle) -> Self {
        let qr_width = QrCode::size();
        let middle_padding = 2;
        let mut text_width = bounds.size.width - middle_padding - qr_width;

        let mogee = MogeeTextStyle::new(BinaryColor::On);
        let style = TextBoxStyle::with_height_mode(HeightMode::FitToText);
        let mut text_height = style.measure_text_height(&mogee, ABOUT, text_width);
        if text_height > bounds.size.height {
            text_width -= Scrolling::bar_allowance();
            // Additional padding between scrollbar & qr code
            text_width -= 1;
            text_height = style.measure_text_height(&mogee, ABOUT, text_width);
        };

        let text_bounds =
            Rectangle::new(bounds.top_left, Size::new(text_width, bounds.size.height));
        let text = TextBox::with_textbox_style(ABOUT, text_bounds, mogee, style);
        let scrolling = Scrolling::new(text_height, bounds);

        let qr_x = bounds.top_left.x + (text_width + middle_padding) as i32;
        let qr_bounds = Rectangle::new(
            Point::new(qr_x, bounds.top_left.y),
            Size::new(qr_width, bounds.size.height),
        );
        // Bounds width is equal to the qr code's actual width, so no need to center
        // horizontally
        let qr_anchor = AnchorPoint::BottomLeft;
        let qr_code = QrCode::new(QR_URL, qr_bounds, qr_anchor).unwrap();

        Self {
            text_bounds,
            text,
            qr_code,
            scrolling,
        }
    }
}

impl Component for About {
    fn init<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.qr_code.init(target)
    }
}

impl View for About {
    fn title(&self) -> &'static str {
        "About VerTX"
    }

    fn input(&mut self, input: Input) -> StateChange {
        let scroll_step = LINE_HEIGHT as i32;
        match input {
            Input::Up => {
                self.scrolling.scroll_by(-scroll_step);
                StateChange::Update
            }
            Input::Down => {
                self.scrolling.scroll_by(scroll_step);
                StateChange::Update
            }
            Input::Forward => StateChange::None,
            Input::Back => StateChange::Pop,
        }
    }
}

impl Drawable for About {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let scroll_offset = self.scrolling.draw(target)? as i32;

        target.cropped(&self.text_bounds).clear(BinaryColor::Off)?;

        let target = &mut target.translated(Point::new(0, -scroll_offset));
        let mut clip = self.text_bounds;
        clip.top_left.y += scroll_offset;
        let target = &mut target.clipped(&clip);
        self.text.draw(target)?;

        Ok(())
    }
}

impl fmt::Debug for About {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("About")
            .field("text_bounds", &self.text_bounds)
            .field("qr_code", &self.qr_code)
            .field("scrolling", &self.scrolling)
            .finish_non_exhaustive()
    }
}
