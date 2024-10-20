#![no_std]

extern crate alloc;

pub mod api;

use heapless::String;
use serde::{Deserialize, Serialize};

pub use self::api::Api;

pub type Ssid = String<32>;
pub type Password = String<64>;

#[derive(Debug, Deserialize, Serialize)]
pub enum Config {
    Home {
        ssid: Ssid,
        password: Password,
        hostname: String<32>,
    },
    Field {
        ssid: Ssid,
        password: Password,
        address: [u8; 4],
    },
}

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
