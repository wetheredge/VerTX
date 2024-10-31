mod backpack;

use std::convert::Infallible;
use std::panic;

use base64::engine::general_purpose::STANDARD_NO_PAD as base64;
use base64::Engine as _;
use embassy_executor::Spawner;
use embassy_sync::pipe::Pipe;
use embedded_graphics as eg;
use smart_leds::RGB8;

mod ipc {
    use std::boxed::Box;
    use std::string::String;

    use wasm_bindgen::prelude::*;

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
    }

    #[wasm_bindgen(js_name = "backpackTx")]
    pub fn backpack_rx(data: Box<[u8]>) {
        let mut remaining = &data[..];
        while !remaining.is_empty() {
            let len = super::BACKPACK_RX.try_write(&data).unwrap();
            remaining = &remaining[len..];
        }
    }
}

#[global_allocator]
/// SAFETY: The runtime environment must be single-threaded WASM.
static ALLOCATOR: talc::TalckWasm = unsafe { talc::TalckWasm::new_global() };

static BACKPACK_RX: backpack::RxPipe = Pipe::new();

declare_hal_types!();

pub(super) fn init(_spawner: Spawner) -> super::Init {
    super::Init {
        reset: Reset,
        led_driver: LedDriver,
        config_storage: ConfigStorage,
        ui: Ui,
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

struct Ui;

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
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = eg::Pixel<Self::Color>>,
    {
        todo!()
    }
}

impl super::traits::Ui for Ui {
    type FlushError = ();

    async fn get_input(&mut self) -> crate::ui::Input {
        todo!()
    }

    async fn flush(&mut self) -> Result<(), Self::FlushError> {
        todo!()
    }
}
