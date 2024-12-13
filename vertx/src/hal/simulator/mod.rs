mod backpack;

use std::panic;
use std::sync::Mutex;

use base64::engine::general_purpose::STANDARD_NO_PAD as base64;
use base64::Engine as _;
use display_interface::DisplayError;
use embassy_executor::Spawner;
use embassy_sync::channel::{self, Channel};
use embassy_sync::pipe::Pipe;
use embedded_graphics as eg;
use smart_leds::RGB8;

use crate::ui::Input as UiInput;

mod ipc {
    use std::boxed::Box;
    use std::string::String;

    use wasm_bindgen::prelude::*;

    use super::UiInput;

    #[wasm_bindgen(js_namespace = Vertx)]
    extern "C" {
        #[wasm_bindgen(js_name = "backpackRx")]
        pub fn backpack_tx(data: &[u8]);

        #[wasm_bindgen(js_name = "loadConfig")]
        pub fn load_config() -> Option<String>;
        #[wasm_bindgen(js_name = "saveConfig")]
        pub fn save_config(data: &str);

        #[wasm_bindgen(js_name = "setStatusLed")]
        pub fn set_status_led(r: u8, g: u8, b: u8);

        #[wasm_bindgen(js_name = "powerOff")]
        pub fn power_off(restart: bool);

        #[wasm_bindgen(js_name = "flushDisplay")]
        pub fn flush_display(data: *const u8);
    }

    #[wasm_bindgen(js_name = "backpackTx")]
    pub fn backpack_rx(data: Box<[u8]>) {
        let mut remaining = &data[..];
        while !remaining.is_empty() {
            let len = super::BACKPACK_RX.try_write(&data).unwrap();
            remaining = &remaining[len..];
        }
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

static BACKPACK_RX: backpack::RxPipe = Pipe::new();
static UI_INPUTS: UiInputsChannel = Channel::new();
static FRAMEBUFFER: Mutex<RawFramebuffer> = Mutex::new(bytemuck::zeroed());

declare_hal_types!();

pub(super) fn init(_spawner: Spawner) -> super::Init {
    super::Init {
        reset: Reset,
        led_driver: LedDriver,
        config_storage: ConfigStorage,
        ui: Ui::new(&FRAMEBUFFER, UI_INPUTS.receiver()),
        backpack: super::Backpack {
            tx: backpack::Tx,
            rx: backpack::Rx(&BACKPACK_RX),
        },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LedBufferOverflow;

#[derive(Debug)]
struct LedDriver;

impl smart_leds::SmartLedsWrite for LedDriver {
    type Color = RGB8;
    type Error = LedBufferOverflow;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        let mut iterator = iterator.into_iter();
        let RGB8 { r, g, b } = iterator.next().unwrap().into();
        ipc::set_status_led(r, g, b);

        if iterator.next().is_some() {
            Err(LedBufferOverflow)
        } else {
            Ok(())
        }
    }
}

struct ConfigStorage;

impl super::traits::ConfigStorage for ConfigStorage {
    fn load<T>(&self, parse: impl FnOnce(&[u8]) -> Option<T>) -> Option<T> {
        ipc::load_config().and_then(|data| {
            let data = base64
                .decode(data)
                .inspect_err(|err| loog::warn!("Failed to decode base64 config: {err:?}"))
                .ok()?;
            parse(&data)
        })
    }

    fn save(&mut self, config: &[u8]) {
        ipc::save_config(&base64.encode(config));
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
