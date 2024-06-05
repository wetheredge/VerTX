use std::thread;

use portable_atomic::AtomicU32;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_env("VERTX_LOG")
        .init();
    log::info!("Starting simulator");

    dbg!(config_len());

    let handle = thread::spawn(|| -> ! {
        let executor = leak(embassy_executor::Executor::new());
        executor.run(|spawner| {
            vertx::main(spawner, leak(AtomicU32::new(0)));
        })
    });
    let _err = handle.join().unwrap_err();

    dbg!(config_len());
}

fn leak<T>(x: T) -> &'static mut T {
    Box::leak(Box::new(x))
}

fn config_len() -> u32 {
    let config = vertx::hal::CONFIG.lock().unwrap();
    config[0]
}
