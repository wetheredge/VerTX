mod configurator;
mod storage;

use std::convert::Infallible;
use std::panic;
use std::sync::Mutex;

use display_interface::DisplayError;
use embassy_executor::Spawner;
use embassy_sync::channel::{self, Channel};
use embedded_graphics as eg;

use crate::ui::Input as UiInput;

mod ipc {
    use std::boxed::Box;
    use std::string::String;
    use std::vec::Vec;

    use wasm_bindgen::prelude::*;

    use super::UiInput;
    use super::configurator::WasmMethod;

    #[wasm_bindgen(js_namespace = Vertx)]
    extern "C" {
        #[wasm_bindgen(js_name = "openConfigurator")]
        pub fn open_configurator();

        #[wasm_bindgen(js_name = "apiRx")]
        pub fn api_tx(id: u32, status: u16, json: bool, body: &[u8]);

        #[wasm_bindgen(js_name = "storageRead")]
        pub fn storage_read(path: &str) -> Option<Vec<u8>>;

        #[wasm_bindgen(js_name = "storageWrite")]
        pub fn storage_write(path: &str, data: &[u8]);

        #[wasm_bindgen(js_name = "setStatusLed")]
        pub fn set_status_led(r: u8, g: u8, b: u8);

        #[wasm_bindgen(js_name = "powerOff")]
        pub fn power_off(restart: bool);

        #[wasm_bindgen(js_name = "flushDisplay")]
        pub fn flush_display(data: *const u8);
    }

    #[wasm_bindgen(js_name = "apiTx")]
    pub fn api_rx(id: u32, route: String, method: WasmMethod, body: Box<[u8]>) {
        super::configurator::Request {
            id,
            route,
            method,
            body,
        }
        .push();
    }

    #[wasm_bindgen(js_name = "buttonPressed")]
    pub fn button_pressed(raw: u8) {
        let input = match raw {
            0 => UiInput::Up,
            1 => UiInput::Down,
            2 => UiInput::Forward,
            3 => UiInput::Back,
            _ => panic!("Invalid button: {raw}"),
        };
        super::UI_INPUTS.try_send(input).unwrap();
    }
}

#[global_allocator]
/// SAFETY: The runtime environment must be single-threaded WASM.
static ALLOCATOR: talc::TalckWasm = unsafe { talc::TalckWasm::new_global() };

type UiInputsChannel = Channel<crate::mutex::MultiCore, UiInput, 10>;
type UiInputsRx = channel::Receiver<'static, crate::mutex::MultiCore, UiInput, 10>;

type RawFramebuffer = [u128; 64];

static UI_INPUTS: UiInputsChannel = Channel::new();
static FRAMEBUFFER: Mutex<RawFramebuffer> = Mutex::new(bytemuck::zeroed());

declare_hal_types!();

pub(super) fn init(_spawner: Spawner) -> super::Init {
    super::Init {
        reset: Reset,
        status_led: StatusLed,
        storage: async { storage::Storage },
        ui: Ui::new(&FRAMEBUFFER, UI_INPUTS.receiver()),
        configurator: configurator::Configurator,
    }
}

struct Reset;

impl super::traits::Reset for Reset {
    fn shut_down(&mut self) -> ! {
        ipc::power_off(false);
        let _ = panic::take_hook();
        panic!()
    }

    fn reboot(&mut self) -> ! {
        ipc::power_off(true);
        let _ = panic::take_hook();
        panic!()
    }
}

#[derive(Debug)]
struct StatusLed;

impl super::traits::StatusLed for StatusLed {
    type Error = Infallible;

    async fn set(&mut self, red: u8, green: u8, blue: u8) -> Result<(), Self::Error> {
        ipc::set_status_led(red, green, blue);
        Ok(())
    }
}

struct Ui {
    framebuffer: &'static Mutex<RawFramebuffer>,
    inputs: UiInputsRx,
}

impl Ui {
    fn new(framebuffer: &'static Mutex<RawFramebuffer>, inputs: UiInputsRx) -> Self {
        Self {
            framebuffer,
            inputs,
        }
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

        let mut data = self.framebuffer.lock().unwrap();

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

impl super::traits::Ui for Ui {
    async fn get_input(&mut self) -> crate::ui::Input {
        self.inputs.receive().await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        let data = self.framebuffer.lock().unwrap();
        ipc::flush_display(data.as_ptr().cast());
        Ok(())
    }
}
