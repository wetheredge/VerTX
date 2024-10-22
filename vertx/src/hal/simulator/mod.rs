mod backpack;

use std::future::Future;
use std::panic;

use embassy_executor::Spawner;
use embassy_sync::pipe::Pipe;
use embassy_sync::signal::Signal;
use smart_leds::RGB8;

mod ipc {
    use std::boxed::Box;

    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(js_namespace = Vertx)]
    extern "C" {
        #[wasm_bindgen(js_name = "backpackRx")]
        pub fn backpack_tx(data: &[u8]);

        #[wasm_bindgen(js_name = "loadConfig")]
        pub fn load_config() -> Option<Box<[u8]>>;
        #[wasm_bindgen(js_name = "saveConfig")]
        pub fn save_config(data: &[u8]);

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

    #[wasm_bindgen(js_name = "modeButtonPressed")]
    pub fn mode_button_pressed() {
        super::MODE_BUTTON.signal(());
    }
}

static BACKPACK_RX: backpack::RxPipe = Pipe::new();
static MODE_BUTTON: Signal<crate::mutex::MultiCore, ()> = Signal::new();

pub(crate) fn init(_spawner: Spawner) -> super::Init {
    super::Init {
        reset: Reset,
        led_driver: LedDriver,
        config_storage: ConfigStorage,
        mode_button: ModeButton(&MODE_BUTTON),
        backpack: backpack::new(&BACKPACK_RX),
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
        ipc::load_config().and_then(|data| parse(&data))
    }

    fn save(&mut self, config: &crate::config::Manager) {
        let mut buffer = [0; crate::config::BYTE_LENGTH];
        let len = config.serialize(&mut buffer).unwrap();
        ipc::save_config(&buffer[0..len]);
    }
}

struct ModeButton(&'static Signal<crate::mutex::MultiCore, ()>);

impl super::traits::ModeButton for ModeButton {
    fn wait_for_pressed(&mut self) -> impl Future<Output = ()> {
        self.0.wait()
    }
}
