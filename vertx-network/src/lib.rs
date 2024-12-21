#![no_std]

use core::net::Ipv4Addr;

use heapless::String;
use serde::{Deserialize, Serialize};

pub type Ssid = String<32>;
pub type Password = String<64>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkKind {
    Home,
    Field,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Credentials {
    pub ssid: Ssid,
    pub password: Password,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HomeConfig {
    pub credentials: Credentials,
    pub hostname: String<32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FieldConfig {
    pub credentials: Credentials,
    pub address: Ipv4Addr,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub home: Option<HomeConfig>,
    pub field: FieldConfig,
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

    #[expect(async_fn_in_trait)]
    async fn start(
        self,
        home: Option<Credentials>,
        field: Credentials,
    ) -> (NetworkKind, Self::Driver);
}
