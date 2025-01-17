#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

#[cfg_attr(feature = "chip-esp", esp_hal_embassy::main)]
#[cfg_attr(feature = "chip-rp", embassy_executor::main)]
async fn main(spawner: embassy_executor::Spawner) {
    vertx::main(spawner).await;
}
