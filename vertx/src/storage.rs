use block_device_adapters::BufStream;
use embassy_executor::task;
use embassy_time::{Delay, Timer};
use embedded_fatfs::{FileSystem, FormatVolumeOptions, FsOptions};
use embedded_hal_bus::spi::ExclusiveDevice;
use fugit::RateExtU32 as _;
use sdspi::SdSpi;

use crate::hal::prelude::*;

#[task]
pub(crate) async fn run(
    init: &'static crate::InitCounter,
    mut spi: crate::hal::SpiBus,
    mut cs: crate::hal::SpiChipSelect,
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

    let mut raw = BufStream::<_, 512>::new(sd);
    let fs_options = FsOptions::new();
    let first_try = FileSystem::new(&mut raw, fs_options).await;
    let fs = match first_try {
        Ok(fs) => fs,
        Err(embedded_fatfs::Error::CorruptedFileSystem) => {
            // Fixes error about mutable aliasing when retrying `FileSystem::new`
            drop(first_try);

            loog::warn!("Formatting SD card");

            let options = FormatVolumeOptions::new().volume_label(*b"VerTX cfg\0\0");
            embedded_fatfs::format_volume(&mut raw, options)
                .await
                .unwrap();

            FileSystem::new(&mut raw, fs_options).await.unwrap()
        }
        Err(err) => {
            loog::panic!("SD card error: {err:?}");
        }
    };

    loog::debug!("FS stats: {:?}", fs.stats().await.unwrap());

    init.finish();
    core::future::pending().await
}
