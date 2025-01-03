use block_device_adapters::BufStream;
use embassy_executor::task;
use embassy_time::{Delay, Timer};
use embedded_fatfs::{DefaultTimeProvider, FileSystem, FsOptions, LossyOemCpConverter};
use embedded_hal_bus::spi::ExclusiveDevice;
use fugit::RateExtU32 as _;
use sdspi::SdSpi;
use static_cell::StaticCell;

use crate::hal::prelude::*;

type Io = BufStream<
    SdSpi<
        ExclusiveDevice<crate::hal::SpiBus, crate::hal::SpiChipSelect, Delay>,
        Delay,
        aligned::A1,
    >,
    512,
>;
type Fs = FileSystem<&'static mut Io, DefaultTimeProvider, LossyOemCpConverter>;
pub(crate) type File =
    embedded_fatfs::File<'static, &'static mut Io, DefaultTimeProvider, LossyOemCpConverter>;

#[task]
pub(crate) async fn run(
    init: &'static crate::InitCounter,
    mut spi: crate::hal::SpiBus,
    mut cs: crate::hal::SpiChipSelect,
    config_manager: &'static crate::config::Manager,
) -> ! {
    let init = init.start(loog::intern!("storage"));

    // Initialize at max speed of 400kHz
    loog::unwrap!(spi.set_config(&crate::hal::SpiConfig::new(400u32.kHz())));

    while let Err(err) = sdspi::sd_init(&mut spi, &mut cs).await {
        loog::warn!("SD card init delay error: {err:?}");
        Timer::after_millis(10).await;
    }

    let spi = ExclusiveDevice::new(spi, cs, Delay).unwrap();

    let mut sd = SdSpi::<_, _, aligned::A1>::new(spi, Delay);
    while let Err(err) = sd.init().await {
        loog::warn!("SD card init error: {err:?}");
        Timer::after_micros(5).await;
    }

    // Run at max speed of 25MHz
    loog::unwrap!(
        sd.spi()
            .bus_mut()
            .set_config(&crate::hal::SpiConfig::new(25u32.MHz()))
    );

    static BUF_STREAM: StaticCell<Io> = StaticCell::new();
    let raw = BUF_STREAM.init_with(|| BufStream::<_, 512>::new(sd));

    static FS: StaticCell<Fs> = StaticCell::new();
    let fs = FileSystem::new(raw, FsOptions::new()).await.unwrap();
    let fs = FS.init(fs);

    let root = fs.root_dir();

    let config_path = "config.bin";
    let config = match root.open_file(config_path).await {
        Ok(file) => file,
        Err(embedded_fatfs::Error::NotFound) => {
            loog::warn!("Creating missing config file");
            root.create_file(config_path).await.unwrap()
        }
        Err(err) => loog::panic!("Failed to open config file: {err:?}"),
    };
    config_manager.load(config).await;

    init.finish();
    core::future::pending().await
}
