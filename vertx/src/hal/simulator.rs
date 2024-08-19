use std::future::Future;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::vec::Vec;
use std::{env, fs, process, thread};

use embassy_executor::Spawner;
use embassy_net_tuntap::TunTapDevice;
use embassy_sync::signal::Signal;
use rand::RngCore as _;
use smart_leds::RGB8;
use vertx_network::{Password, Ssid};
use vertx_simulator_ipc as ipc;

pub(crate) fn init(_spawner: Spawner) -> super::Init {
    static MODE_BUTTON: Signal<crate::mutex::MultiCore, ()> = Signal::new();
    thread::Builder::new()
        .name("ipc rx".into())
        .spawn(move || {
            let mut stdin = BufReader::new(io::stdin());
            let mut message = Vec::new();
            while stdin.read_until(0, &mut message).unwrap() > 0 {
                let message = ipc::deserialize(&mut message).unwrap();
                match message {
                    ipc::ToFirmware::ModeButtonPressed => MODE_BUTTON.signal(()),
                }
            }
        })
        .unwrap();

    super::Init {
        rng: Rng::new(),
        boot_mode: env::var("VERTX_BOOT_MODE")
            .map(|mode| mode.parse::<u8>().unwrap().into())
            .unwrap_or_default(),
        led_driver: LedDriver,
        config_storage: ConfigStorage::new(),
        mode_button: ModeButton(&MODE_BUTTON),
        network: Network,
    }
}

pub(crate) fn set_boot_mode(mode: u8) {
    ipc_send(ipc::ToManager::ChangeMode(mode));
}

pub(crate) fn shut_down() -> ! {
    ipc_send(ipc::ToManager::ShutDown);
    process::exit(0);
}

pub(crate) fn reboot() -> ! {
    ipc_send(ipc::ToManager::Reboot);
    process::exit(0);
}

pub(crate) fn get_cycle_count() -> u32 {
    0
}

fn ipc_send(message: ipc::ToManager) {
    let bytes = ipc::serialize(&message).unwrap();
    let mut stdout = io::stdout();
    stdout.write_all(&bytes).unwrap();
    stdout.flush().unwrap();
}

#[derive(Clone)]
struct Rng(rand::rngs::ThreadRng);

impl Rng {
    fn new() -> Self {
        Self(rand::thread_rng())
    }
}

impl super::traits::Rng for Rng {
    fn u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    fn u64(&mut self) -> u64 {
        self.0.next_u64()
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
        ipc_send(ipc::ToManager::StatusLed { r, g, b });

        if iterator.next().is_some() {
            Err(LedBufferOverflow)
        } else {
            Ok(())
        }
    }
}

struct ConfigStorage(PathBuf);

impl ConfigStorage {
    fn new() -> Self {
        let path = env::var_os("VERTX_CONFIG").expect(
            "VERTX_CONFIG env var should be set to the path for the simulator config storage",
        );
        Self(path.into())
    }
}

impl super::traits::ConfigStorage for ConfigStorage {
    fn load<T>(&self, parse: impl FnOnce(&[u8]) -> T) -> Option<T> {
        match fs::read(&self.0) {
            Ok(contents) => Some(parse(&contents)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) => panic!("Failed to read config file: {err:?}"),
        }
    }

    fn save(&mut self, data: &[u32]) {
        fs::write(&self.0, bytemuck::cast_slice(data)).unwrap();
    }
}

struct ModeButton(&'static Signal<crate::mutex::MultiCore, ()>);

impl super::traits::ModeButton for ModeButton {
    fn wait_for_pressed(&mut self) -> impl Future<Output = ()> {
        self.0.wait()
    }
}

struct Network;

impl vertx_network::Hal for Network {
    type Driver = TunTapDevice;

    const SUPPORTS_FIELD: bool = true;

    fn field(self, _ssid: Ssid, _password: Password) -> Self::Driver {
        let interface = match env::var("VERTX_NET_INTERFACE") {
            Ok(interface) => interface,
            Err(env::VarError::NotPresent) => {
                panic!("missing VERTX_NET_INTERFACE environment variable")
            }
            Err(_) => panic!("Failed to parse VERTX_NET_INTERFACE contents"),
        };
        TunTapDevice::new(&interface).unwrap()
    }
}
