#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]

use embassy_executor::{main, Spawner};

#[main]
async fn main(spawner: Spawner) {
    env_logger::builder()
        .filter_level(loog::log::LevelFilter::Debug)
        .parse_env("VERTX_LOG")
        .init();

    vertx::main(spawner).await;
}
