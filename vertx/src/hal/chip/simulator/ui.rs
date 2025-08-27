use std::sync::Mutex;

use display_interface::DisplayError;
use embassy_sync::channel::{self, Channel};
use embedded_graphics as eg;

use crate::hal;
use crate::ui::Input;

pub(super) type RawFramebuffer = [u128; 64];
pub(super) type InputsChannel = Channel<crate::mutex::MultiCore, Input, 10>;
pub(super) type InputsRx = channel::Receiver<'static, crate::mutex::MultiCore, Input, 10>;

static FRAMEBUFFER: Mutex<RawFramebuffer> = Mutex::new(bytemuck::zeroed());

pub(super) struct Ui {
    inputs: InputsRx,
}

impl Ui {
    pub(super) fn new(inputs: InputsRx) -> Self {
        Self { inputs }
    }
}

impl eg::geometry::OriginDimensions for Ui {
    fn size(&self) -> eg::geometry::Size {
        eg::geometry::Size {
            width: 128,
            height: 64,
        }
    }
}

impl eg::draw_target::DrawTarget for Ui {
    type Color = eg::pixelcolor::BinaryColor;
    type Error = DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = eg::Pixel<Self::Color>>,
    {
        use eg::geometry::Point;

        let mut data = FRAMEBUFFER.lock().unwrap();

        for eg::Pixel(Point { x, y }, color) in pixels {
            if (0..64).contains(&y) && (0..128).contains(&x) {
                let color = color == Self::Color::On;
                let col = x as u128;
                let row = y as usize;
                data[row] = data[row] & !(1 << col) | (u128::from(color) << col);
            }
        }

        Ok(())
    }
}

impl hal::traits::Ui for Ui {
    async fn get_input(&mut self) -> crate::ui::Input {
        self.inputs.receive().await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        let data = FRAMEBUFFER.lock().unwrap();
        super::ipc::flush_display(data.as_ptr().cast());
        Ok(())
    }
}
