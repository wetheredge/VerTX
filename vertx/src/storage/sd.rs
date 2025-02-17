use block_device_adapters::BufStream;
use delegate::delegate;
use embassy_time::{Delay, Timer};
use embedded_fatfs::FileSystem;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::{SpiBus, SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_io_async::{ErrorType, Read, ReadExactError, Seek, SeekFrom, Write};
use fugit::RateExtU32 as _;
use sdspi::SdSpi;

use super::pal;

type Io<S> = BufStream<SdSpi<S, Delay, aligned::A1>, 512>;
type TimeProvider = embedded_fatfs::NullTimeProvider;
type PathConverter = embedded_fatfs::LossyOemCpConverter;

type FsError<S> = embedded_fatfs::Error<<Io<S> as ErrorType>::Error>;

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

impl<S: SpiDevice> ErrorType for &Storage<S> {
    type Error = FsError<S>;
}

impl<'a, S: SpiDevice> pal::Storage for &'a Storage<S> {
    type Directory = Directory<'a, S>;

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

impl<S: SpiDevice> ErrorType for Directory<'_, S> {
    type Error = FsError<S>;
}

impl<'a, S: SpiDevice> pal::Directory for Directory<'a, S> {
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

impl<S: SpiDevice> ErrorType for File<'_, S> {
    type Error = FsError<S>;
}

impl<S: SpiDevice> Seek for File<'_, S> {
    delegate! {
        to self.inner {
            async fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error>;
            async fn rewind(&mut self) -> Result<(), Self::Error>;
            async fn stream_position(&mut self) -> Result<u64, Self::Error>;
        }
    }
}

impl<S: SpiDevice> Read for File<'_, S> {
    delegate! {
        to self.inner {
            async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
            async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), ReadExactError<Self::Error>>;
        }
    }
}

impl<S: SpiDevice> Write for File<'_, S> {
    delegate! {
        to self.inner {
            async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
            async fn flush(&mut self) -> Result<(), Self::Error>;
            async fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
        }
    }
}

impl<S: SpiDevice> pal::File for File<'_, S> {
    async fn truncate(&mut self) -> Result<(), Self::Error> {
        self.inner.truncate().await
    }

    async fn close(self) -> Result<(), Self::Error> {
        self.inner.close().await?;
        Ok(())
    }
}

impl<S: SpiDevice> ErrorType for DirectoryIter<'_, S> {
    type Error = FsError<S>;
}

impl<'a, S: SpiDevice> pal::DirectoryIter for DirectoryIter<'a, S> {
    type Directory = Directory<'a, S>;
    type Entry = Entry<'a, S>;
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

impl<S: SpiDevice> ErrorType for Entry<'_, S> {
    type Error = FsError<S>;
}

impl<'a, S: SpiDevice> pal::Entry for Entry<'a, S> {
    type Directory = Directory<'a, S>;
    type File = File<'a, S>;

    delegate! {
        to self.inner {
            #[call(short_file_name_as_bytes)]
            fn name(&self) -> &[u8];

            fn is_file(&self) -> bool;
            fn is_dir(&self) -> bool;
        }
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
