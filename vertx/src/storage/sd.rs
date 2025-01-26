use block_device_adapters::BufStream;
use embassy_time::{Delay, Timer};
use embedded_fatfs::FileSystem;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::{SpiBus, SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_io_async::{Read as _, Seek as _, SeekFrom, Write as _};
use fugit::RateExtU32 as _;
use sdspi::SdSpi;

use super::pal;

type Io<S> = BufStream<SdSpi<S, Delay, aligned::A1>, 512>;
type TimeProvider = embedded_fatfs::NullTimeProvider;
type PathConverter = embedded_fatfs::LossyOemCpConverter;

type FsError<S> = embedded_fatfs::Error<<Io<S> as embedded_io_async::ErrorType>::Error>;

pub(crate) struct Storage<S: SpiDevice>(FileSystem<Io<S>, TimeProvider, PathConverter>);

pub(crate) struct Directory<'a, S: SpiDevice>(
    embedded_fatfs::Dir<'a, Io<S>, TimeProvider, PathConverter>,
);

pub(crate) struct File<'a, S: SpiDevice>(
    embedded_fatfs::File<'a, Io<S>, TimeProvider, PathConverter>,
);

impl<S: SpiDevice> Clone for File<'_, S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<B, CS, E> Storage<ExclusiveDevice<B, CS, embassy_time::Delay>>
where
    B: SpiBus,
    CS: OutputPin<Error = E>,
    E: loog::DebugFormat,
{
    pub(crate) async fn new_exclusive_spi(
        mut bus: B,
        mut cs: CS,
        set_speed: impl Fn(&mut B, fugit::HertzU32),
    ) -> Self {
        set_speed(&mut bus, 400u32.kHz());

        while let Err(err) = sdspi::sd_init(&mut bus, &mut cs).await {
            loog::warn!("SD card init delay error: {err:?}");
            Timer::after_millis(10).await;
        }

        let spi = loog::unwrap!(ExclusiveDevice::new(bus, cs, embassy_time::Delay));

        let mut sd = sdspi::SdSpi::new(spi, Delay);
        while let Err(err) = sd.init().await {
            loog::warn!("SD card init error: {err:?}");
            Timer::after_millis(5).await;
        }

        set_speed(sd.spi().bus_mut(), 25u32.MHz());

        let buf_stream = BufStream::new(sd);
        let fs = FileSystem::new(buf_stream, embedded_fatfs::FsOptions::new())
            .await
            .unwrap();

        Self(fs)
    }
}

impl<S: SpiDevice> pal::StorageError for &'_ Storage<S> {
    type Error = FsError<S>;
}

impl<'a, S: SpiDevice> pal::Storage for &'a Storage<S> {
    type Directory = Directory<'a, S>;

    fn root(&self) -> Self::Directory {
        Directory(self.0.root_dir())
    }

    async fn flush(&self) -> Result<(), Self::Error> {
        todo!()
    }
}

impl<S: SpiDevice> pal::StorageError for Directory<'_, S> {
    type Error = FsError<S>;
}

impl<'a, S: SpiDevice> pal::Directory for Directory<'a, S> {
    type File = File<'a, S>;

    async fn dir(&self, path: &str) -> Result<Self, Self::Error> {
        match self.0.open_dir(path).await {
            Ok(dir) => Ok(Self(dir)),
            Err(Self::Error::NotFound) => Ok(Self(self.0.create_dir(path).await?)),
            Err(err) => Err(err),
        }
    }

    async fn file(&self, path: &str) -> Result<Self::File, Self::Error> {
        match self.0.open_file(path).await {
            Ok(file) => Ok(File(file)),
            Err(Self::Error::NotFound) => Ok(File(self.0.create_file(path).await?)),
            Err(err) => Err(err),
        }
    }
}

impl<S: SpiDevice> pal::StorageError for File<'_, S> {
    type Error = FsError<S>;
}

impl<S: SpiDevice> pal::File for File<'_, S> {
    async fn read_all(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.seek(SeekFrom::Start(0)).await?;

        let mut len = 0;
        while len < buffer.len() {
            len += self.0.read(buffer).await?;
        }

        Ok(len)
    }

    async fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
        self.0.seek(SeekFrom::Start(0)).await?;
        self.0.truncate().await?;
        self.0.write_all(buffer).await
    }
}
