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

type StorageInner<S> = FileSystem<Io<S>, TimeProvider, PathConverter>;
pub(crate) struct Storage<S: SpiDevice>(StorageInner<S>);

pub(crate) struct Directory<'a, S: SpiDevice> {
    storage: &'a StorageInner<S>,
    inner: embedded_fatfs::Dir<'a, Io<S>, TimeProvider, PathConverter>,
}

impl<S: SpiDevice> Clone for Directory<'_, S> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage,
            inner: self.inner.clone(),
        }
    }
}

pub(crate) struct File<'a, S: SpiDevice> {
    storage: &'a StorageInner<S>,
    inner: embedded_fatfs::File<'a, Io<S>, TimeProvider, PathConverter>,
}

impl<S: SpiDevice> Clone for File<'_, S> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage,
            inner: self.inner.clone(),
        }
    }
}

pub(crate) struct DirectoryIter<'a, S: SpiDevice> {
    storage: &'a StorageInner<S>,
    inner: embedded_fatfs::DirIter<'a, Io<S>, TimeProvider, PathConverter>,
}

impl<'a, S: SpiDevice> DirectoryIter<'a, S> {
    pub(crate) fn new(
        storage: &'a StorageInner<S>,
        inner: embedded_fatfs::DirIter<'a, Io<S>, TimeProvider, PathConverter>,
    ) -> Self {
        Self { storage, inner }
    }
}

impl<S: SpiDevice> Clone for DirectoryIter<'_, S> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage,
            inner: self.inner.clone(),
        }
    }
}

pub(crate) struct Entry<'a, S: SpiDevice> {
    storage: &'a StorageInner<S>,
    inner: embedded_fatfs::DirEntry<'a, Io<S>, TimeProvider, PathConverter>,
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

impl<'a, S: SpiDevice> pal::Storage for &'a Storage<S> {
    type Directory = Directory<'a, S>;
    type Error = FsError<S>;

    const FILENAME_BYTES: usize = 12;

    fn root(&self) -> Self::Directory {
        Directory {
            storage: &self.0,
            inner: self.0.root_dir(),
        }
    }

    async fn flush(&self) -> Result<(), Self::Error> {
        self.0.flush().await
    }
}

impl<'a, S: SpiDevice> pal::Directory for Directory<'a, S> {
    type Error = FsError<S>;
    type File = File<'a, S>;
    type Iter = DirectoryIter<'a, S>;

    async fn dir(&self, path: &str) -> Result<Self, Self::Error> {
        match self.inner.open_dir(path).await {
            Ok(dir) => Ok(Self {
                storage: self.storage,
                inner: dir,
            }),
            Err(Self::Error::NotFound) => Ok(Self {
                storage: self.storage,
                inner: self.inner.create_dir(path).await?,
            }),
            Err(err) => Err(err),
        }
    }

    async fn file(&self, path: &str) -> Result<Self::File, Self::Error> {
        match self.inner.open_file(path).await {
            Ok(file) => Ok(File {
                storage: self.storage,
                inner: file,
            }),
            Err(Self::Error::NotFound) => Ok(File {
                storage: self.storage,
                inner: self.inner.create_file(path).await?,
            }),
            Err(err) => Err(err),
        }
    }

    fn iter(&self) -> Self::Iter {
        DirectoryIter {
            storage: self.storage,
            inner: self.inner.iter(),
        }
    }
}

impl<S: SpiDevice> pal::File for File<'_, S> {
    type Error = FsError<S>;

    async fn seek_to_start(&mut self) -> Result<(), Self::Error> {
        self.inner.seek(SeekFrom::Start(0)).await?;
        Ok(())
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.inner.read(buffer).await
    }

    async fn read_exact(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(), embedded_io_async::ReadExactError<Self::Error>> {
        self.inner.read_exact(buffer).await
    }

    async fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
        self.inner.seek(SeekFrom::Start(0)).await?;
        self.inner.truncate().await?;
        self.inner.write_all(buffer).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await
    }
}

impl<'a, S: SpiDevice> pal::DirectoryIter for DirectoryIter<'a, S> {
    type Directory = Directory<'a, S>;
    type Entry = Entry<'a, S>;
    type Error = FsError<S>;
    type File = File<'a, S>;

    async fn next(&mut self) -> Option<Result<Self::Entry, Self::Error>> {
        self.inner.next().await.map(|res| {
            res.map(|inner| Entry {
                storage: self.storage,
                inner,
            })
        })
    }
}

impl<'a, S: SpiDevice> pal::Entry for Entry<'a, S> {
    type Directory = Directory<'a, S>;
    type Error = FsError<S>;
    type File = File<'a, S>;

    fn name(&self) -> &[u8] {
        self.inner.short_file_name_as_bytes()
    }

    fn to_file(self) -> Option<Self::File> {
        self.inner.is_file().then(|| File {
            storage: self.storage,
            inner: self.inner.to_file(),
        })
    }

    fn to_dir(self) -> Option<Self::Directory> {
        self.inner.is_dir().then(|| Directory {
            storage: self.storage,
            inner: self.inner.to_dir(),
        })
    }
}
