fn main() {
    env_logger::builder()
        .filter_level(loog::log::LevelFilter::Info)
        .parse_env("VERTX_LOG")
        .init();

    let executor = leak(embassy_executor::Executor::new());
    executor.run(|spawner| {
        vertx::main(spawner, leak(portable_atomic::AtomicU32::new(0)));
    });
}

fn leak<T>(x: T) -> &'static mut T {
    Box::leak(Box::new(x))
}
