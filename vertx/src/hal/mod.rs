use core::fmt::Debug;

#[cfg_attr(feature = "esp32", path = "esp32/mod.rs")]
#[cfg_attr(feature = "simulator", path = "simulator.rs")]
mod implementation;

#[cfg(feature = "simulator")]
pub use implementation::*;
#[cfg(not(feature = "simulator"))]
pub(crate) use implementation::*;

pub(crate) type Rng = impl traits::Rng;
pub(crate) type LedDriver =
    impl smart_leds::SmartLedsWrite<Error = impl Debug, Color = smart_leds::RGB8>;
pub(crate) type ConfigStorage = impl traits::ConfigStorage;
pub(crate) type ModeButton = impl traits::ModeButton;
pub(crate) type GetModeButton = impl FnOnce() -> ModeButton;
pub(crate) type NetDriver = impl embassy_net::driver::Driver + 'static;
pub(crate) type GetNetDriver =
    impl FnOnce(&'static heapless::String<32>, &'static heapless::String<64>) -> NetDriver;

pub(crate) struct Init {
    pub(crate) rng: Rng,
    pub(crate) led_driver: LedDriver,
    pub(crate) config_storage: ConfigStorage,
    pub(crate) get_mode_button: GetModeButton,
    pub(crate) get_net_driver: GetNetDriver,
}

pub(crate) mod traits {
    use core::future::Future;

    pub(crate) trait ConfigStorage {
        fn load<T>(&self, parse: impl FnOnce(&[u8]) -> T) -> Option<T>;
        fn save(&mut self, data: &[u32]);
    }

    pub(crate) trait Rng: Clone {
        fn u32(&mut self) -> u32;

        fn u64(&mut self) -> u64 {
            (u64::from(self.u32()) << 32) | u64::from(self.u32())
        }
    }

    pub(crate) trait ModeButton {
        fn wait_for_pressed(&mut self) -> impl Future<Output = ()>;
    }
}
