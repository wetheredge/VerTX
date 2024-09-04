mod backpack;

use std::future::Future;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::vec::Vec;
use std::{env, fs, process, thread};

use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use postcard::accumulator::{CobsAccumulator, FeedResult};
use smart_leds::RGB8;
use vertx_simulator_ipc as ipc;

pub(crate) fn init(_spawner: Spawner) -> super::Init {
    static BACKPACK_RX: Channel<crate::mutex::MultiCore, Vec<u8>, 10> = Channel::new();
    static BACKPACK_ACK: backpack::AckSignal = backpack::AckSignal::new();
    static MODE_BUTTON: Signal<crate::mutex::MultiCore, ()> = Signal::new();

    thread::Builder::new()
        .name("ipc rx".into())
        .spawn(move || {
            let mut stdin = io::stdin().lock();

            let mut buffer = [0; 128];
            let mut accumulator: CobsAccumulator<512> = CobsAccumulator::new();

            loop {
                let len = stdin.read(&mut buffer).unwrap();
                let mut chunk = &buffer[..len];

                while !chunk.is_empty() {
                    chunk = match accumulator.feed(chunk) {
                        FeedResult::Consumed => break,
                        FeedResult::OverFull(remaining) => remaining,
                        FeedResult::DeserError(remaining) => {
                            loog::warn!("ipc deserialization failed");
                            remaining
                        }
                        FeedResult::Success { data, remaining } => {
                            match data {
                                ipc::Message::Backpack(chunk) => {
                                    BACKPACK_RX.try_send(chunk.into_owned()).unwrap();
                                }
                                ipc::Message::Simulator(message) => match message {
                                    ipc::ToVertx::BackpackAck => BACKPACK_ACK.signal(()),
                                    ipc::ToVertx::ModeButtonPressed => MODE_BUTTON.signal(()),
                                },
                            }

                            remaining
                        }
                    };
                }
            }
        })
        .unwrap();

    super::Init {
        reset: Reset,
        rng: rand::thread_rng(),
        boot_mode: env::var("VERTX_BOOT_MODE")
            .map(|mode| mode.parse::<u8>().unwrap().into())
            .expect("VERTX_BOOT_MODE env var should be set"),
        led_driver: LedDriver,
        config_storage: ConfigStorage::new(),
        mode_button: ModeButton(&MODE_BUTTON),
        backpack: backpack::new(BACKPACK_RX.receiver(), &BACKPACK_ACK),
    }
}

pub(crate) fn set_boot_mode(mode: u8) {
    ipc_send(ipc::ToManager::SetBootMode(mode));
}

fn ipc_send<'a, M: Into<ipc::Message<'a, ipc::ToManager>>>(message: M) {
    let bytes = ipc::serialize(&message.into()).unwrap();
    let mut stdout = io::stdout();
    stdout.write_all(&bytes).unwrap();
    stdout.flush().unwrap();
}

struct Reset;

impl super::traits::Reset for Reset {
    fn shut_down(&mut self) -> ! {
        process::exit(ipc::EXIT_SHUT_DOWN);
    }

    fn reboot(&mut self) -> ! {
        process::exit(ipc::EXIT_REBOOT);
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

    fn save(&mut self, data: Vec<u8>) {
        fs::write(&self.0, data).unwrap();
    }
}

struct ModeButton(&'static Signal<crate::mutex::MultiCore, ()>);

impl super::traits::ModeButton for ModeButton {
    fn wait_for_pressed(&mut self) -> impl Future<Output = ()> {
        self.0.wait()
    }
}
