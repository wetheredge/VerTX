#[cfg(feature = "storage-sd-spi")]
pub(crate) mod sd_spi;

use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::hal::prelude::*;

pub(crate) mod pal {
    pub(super) use embedded_io_async::ErrorType;
    use embedded_io_async::{Read, Seek, Write};

    pub(crate) trait Storage: ErrorType {
        type File<'s>: File<Error = Self::Error>
        where
            Self: 's;

        async fn read_config<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8], Self::Error>;
        async fn write_config(&mut self, config: &[u8]) -> Result<(), Self::Error>;

        async fn model_names<F>(&mut self, f: F) -> Result<(), Self::Error>
        where
            F: FnMut(crate::models::Id, &str);
        async fn model(
            &mut self,
            id: crate::models::Id,
        ) -> Result<Option<Self::File<'_>>, Self::Error>;
        async fn delete_model(&mut self, id: crate::models::Id) -> Result<(), Self::Error>;

        async fn flush(&mut self) -> Result<(), Self::Error>;
    }

    pub(crate) trait File: Sized + ErrorType + Read + Write + Seek {
        #[expect(unused)]
        async fn len(&mut self) -> u64;
        #[expect(unused)]
        async fn truncate(&mut self) -> Result<(), Self::Error>;

        async fn close(mut self) -> Result<(), Self::Error> {
            self.flush().await
        }
    }
}

pub(crate) type Error = <crate::hal::Storage as embedded_io_async::ErrorType>::Error;
pub(crate) type File<'a> = <crate::hal::Storage as pal::Storage>::File<'a>;

type Inner = Mutex<crate::mutex::MultiCore, crate::hal::Storage>;

pub(crate) async fn init(storage: crate::hal::StorageFuture) -> (Storage, Config, Models) {
    static INNER: StaticCell<Inner> = StaticCell::new();

    let storage = storage.await;
    let inner = INNER.init_with(|| Mutex::new(storage));

    (Storage(&*inner), Config(&*inner), Models(&*inner))
}

pub(crate) struct Storage(&'static Inner);

#[derive(Clone, Copy)]
pub(crate) struct Config(&'static Inner);

#[derive(Clone, Copy)]
pub(crate) struct Models(&'static Inner);

impl Storage {
    pub(crate) async fn flush(self) {
        let mut storage = self.0.lock().await;
        if let Err(err) = storage.flush().await {
            loog::error!("Failed to flush: {err:?}");
        }
    }
}

impl Config {
    pub(crate) async fn read(self, buf: &mut [u8]) -> Result<&[u8], Error> {
        let mut storage = self.0.lock().await;
        storage.read_config(buf).await
    }

    pub(crate) async fn write(self, config: &[u8]) -> Result<(), Error> {
        let mut storage = self.0.lock().await;
        storage.write_config(config).await
    }
}

impl Models {
    pub(crate) async fn names<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnMut(crate::models::Id, &str),
    {
        let mut storage = self.0.lock().await;
        storage.model_names(f).await
    }

    #[expect(unused)]
    pub(crate) async fn model<T>(
        &self,
        id: crate::models::Id,
        mut f: impl AsyncFnMut(&mut File) -> Result<T, Error>,
    ) -> Result<Option<T>, Error> {
        let mut storage = self.0.lock().await;
        match storage.model(id).await {
            Ok(Some(mut file)) => {
                let ret = f(&mut file).await?;
                file.close().await?;
                Ok(Some(ret))
            }
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    #[expect(unused)]
    pub(crate) async fn delete(&self, id: crate::models::Id) -> Result<(), Error> {
        let mut storage = self.0.lock().await;
        storage.delete_model(id).await
    }
}
