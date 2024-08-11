#![no_std]

pub type Ssid = heapless::String<32>;
pub type Password = heapless::String<64>;

pub trait Hal: Sized {
    type Driver: 'static + embassy_net_driver::Driver;

    const SUPPORTS_HOME: bool = false;
    const SUPPORTS_FIELD: bool = false;

    fn home(self, _ssid: Ssid, _password: Password) -> Self::Driver {
        unimplemented!()
    }

    fn field(self, _ssid: Ssid, _password: Password) -> Self::Driver {
        unimplemented!()
    }
}
