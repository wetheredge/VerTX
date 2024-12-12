#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

#[cfg(feature = "chip-rp")]
use embassy_executor::main;
use embassy_executor::Spawner;
#[cfg(feature = "chip-esp")]
use esp_hal::prelude::main;

#[main]
async fn main(spawner: Spawner) {
    vertx::main(spawner).await;
}
