mod configurator;
mod storage;
mod ui;

use std::convert::Infallible;
use std::panic;

use embassy_executor::Spawner;
use embassy_sync::channel::Channel;

use crate::hal;
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
        pub(super) fn open_configurator();

        #[wasm_bindgen(js_name = "apiRx")]
        pub(super) fn api_tx(id: u32, status: u16, json: bool, body: &[u8]);

        #[wasm_bindgen(js_name = "storageFileLength")]
        pub(super) fn storage_file_len(path: &str) -> usize;

        #[wasm_bindgen(js_name = "storageRead")]
        pub(super) fn storage_read(path: &str, cursor: usize, buffer: &mut [u8]) -> usize;

        #[wasm_bindgen(js_name = "storageWrite")]
        pub(super) fn storage_write(path: &str, cursor: usize, data: &[u8]);

        #[wasm_bindgen(js_name = "storageTruncate")]
        pub(super) fn storage_truncate(path: &str, cursor: usize);

        #[wasm_bindgen(js_name = "storageDirEntries")]
        pub(super) fn storage_dir_entries(path: &str) -> Vec<String>;

        #[wasm_bindgen(js_name = "setStatusLed")]
        pub(super) fn set_status_led(r: u8, g: u8, b: u8);

        #[wasm_bindgen(js_name = "powerOff")]
        pub(super) fn power_off(restart: bool);

        #[wasm_bindgen(js_name = "flushDisplay")]
        pub(super) fn flush_display(data: *const u8);
    }

    #[wasm_bindgen(js_name = "apiTx")]
    pub fn api_rx(id: u32, route: String, method: WasmMethod, body: Option<Box<[u8]>>) {
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

static UI_INPUTS: ui::InputsChannel = Channel::new();

#[define_opaque(
    hal::Configurator,
    hal::Reset,
    hal::StatusLed,
    hal::StorageFuture,
    hal::Ui
)]
pub(crate) fn init(_spawner: Spawner) -> hal::Init {
    hal::Init {
        reset: Reset,
        status_led: StatusLed,
        storage: async { storage::Storage },
        ui: ui::Ui::new(UI_INPUTS.receiver()),
        configurator: configurator::Configurator,
    }
}

struct Reset;

impl hal::traits::Reset for Reset {
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

impl hal::traits::StatusLed for StatusLed {
    type Error = Infallible;

    async fn set(&mut self, red: u8, green: u8, blue: u8) -> Result<(), Self::Error> {
        ipc::set_status_led(red, green, blue);
        Ok(())
    }
}
