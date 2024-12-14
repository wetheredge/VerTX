#![no_std]

use core::net::Ipv4Addr;

use heapless::String;
use serde::{Deserialize, Serialize};

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
        address: Ipv4Addr,
    },
}

#[expect(async_fn_in_trait)]
pub trait Api {
    type Buffer;

    fn buffer() -> Self::Buffer;
    async fn next_response<'b>(&self, buffer: &'b mut Self::Buffer) -> &'b [u8];
    async fn handle<'b>(&self, request: &[u8], buffer: &'b mut Self::Buffer) -> Option<&'b [u8]>;
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
