use block_device_driver::BlockDevice;
use delegate::delegate;
use embassy_time::Timer;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiBus;
use embedded_hal_bus::spi::ExclusiveDevice;
use sdspi::SdSpi;
use vertx_filesystem::{BLOCK_BYTES, Buffers, File, Filesystem, InitError};

use super::pal;

pub(crate) async fn new_exclusive_spi<A, B, CS, E>(
    buffers: &mut Buffers<A>,
    mut bus: B,
    mut cs: CS,
    set_speed: impl Fn(&mut B, u32),
) -> Filesystem<'_, SdSpi<ExclusiveDevice<B, CS, embassy_time::Delay>, embassy_time::Delay, A>>
where
    A: aligned::Alignment,
    B: SpiBus,
    CS: OutputPin<Error = E>,
    E: loog::DebugFormat,
{
    set_speed(&mut bus, 400_000);

    while let Err(err) = sdspi::sd_init(&mut bus, &mut cs).await {
        loog::warn!("SD card init delay error: {err:?}");
        Timer::after_millis(10).await;
    }

    let spi = loog::unwrap!(ExclusiveDevice::new(bus, cs, embassy_time::Delay));

    let mut sd = sdspi::SdSpi::new(spi, embassy_time::Delay);
    while let Err(err) = sd.init().await {
        loog::warn!("SD card init error: {err:?}");
        Timer::after_millis(5).await;
    }

    set_speed(sd.spi().bus_mut(), 25_000_000);

    match Filesystem::new(sd, buffers).await {
        Ok(fs) => fs,
        Err(InitError::HeaderError {
            kind,
            device,
            buffers,
        }) => {
            loog::warn!("Filesystem header is invalid ({kind:?}); erasing");
            Filesystem::new_empty(device, buffers)
        }
        Err(InitError::Io(err)) => {
            loog::panic!("IO error while opening filesystem: {err}");
        }
    }
}

impl<'buf, D: BlockDevice<BLOCK_BYTES>> pal::Storage for Filesystem<'buf, D>
where
    D::Error: embedded_io_async::Error,
{
    type File<'s>
        = File<'buf, 's, D>
    where
        Self: 's;

    delegate! {
        to self {
            async fn read_config<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8], Self::Error>;
            async fn write_config(&mut self, config: &[u8]) -> Result<(), Self::Error>;

            async fn model_names<F>(&mut self, f: F) -> Result<(), Self::Error>
            where
                F: FnMut(crate::models::Id, &str);

            async fn model(&mut self, id: crate::models::Id)
                -> Result<Option<Self::File<'_>>, Self::Error>;

            async fn delete_model(&mut self, id: crate::models::Id) -> Result<(), Self::Error>;

            async fn flush(&mut self) -> Result<(), Self::Error>;
        }
    }
}

impl<D: BlockDevice<BLOCK_BYTES>> pal::File for File<'_, '_, D>
where
    D::Error: embedded_io_async::Error,
{
    delegate! {
        to self {
            #[await(false)]
            async fn len(&mut self) -> u64;

            #[await(false)]
            #[expr($; Ok(()))]
            async fn truncate(&mut self) -> Result<(), Self::Error>;

            async fn close(self) -> Result<(), Self::Error>;
        }
    }
}
