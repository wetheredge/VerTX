use std::future;
use std::sync::Mutex;

use embassy_executor::Spawner;
use rand::RngCore as _;

pub static CONFIG: Mutex<[u32; 1024]> = Mutex::new([0; 1024]);
pub static RESET_REASON: Mutex<ResetReason> = Mutex::new(ResetReason::Crash);

pub(crate) fn init(_spawner: Spawner) -> super::Init {
    super::Init {
        rng: Rng::new(),
        led_driver: LedDriver,
        config_storage: ConfigStorage(&CONFIG),
        configurator_button: future::pending(),
        get_net_driver: |_ssid, _password| embassy_net_tuntap::TunTapDevice::new("TODO").unwrap(),
    }
}

pub(crate) fn shut_down() {
    *RESET_REASON.lock().unwrap() = ResetReason::ShutDown;
}

pub(crate) fn reboot() {
    *RESET_REASON.lock().unwrap() = ResetReason::Reboot;
}

pub(crate) fn get_cycle_count() -> u32 {
    0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResetReason {
    Crash,
    ShutDown,
    Reboot,
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
pub(crate) struct LedBufferOverflow;

#[derive(Debug)]
struct LedDriver;

impl smart_leds::SmartLedsWrite for LedDriver {
    type Color = smart_leds::RGB8;
    type Error = LedBufferOverflow;

    fn write<T, I>(&mut self, _iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        // TODO
        Ok(())
    }
}

struct ConfigStorage(&'static Mutex<[u32; 1024]>);

impl super::traits::ConfigStorage for ConfigStorage {
    fn load<T>(&self, parse: impl FnOnce(&[u8]) -> T) -> Option<T> {
        let raw = self.0.lock().unwrap();
        let length = raw[0] as usize;

        (length > 0).then(|| {
            let bytes: &[u8] = &bytemuck::cast_slice(&raw[1..])[..length];
            parse(bytes)
        })
    }

    fn save(&mut self, data: &[u32]) {
        let mut raw = self.0.lock().unwrap();
        raw.fill(0);
        raw[..data.len()].copy_from_slice(data);
    }
}
