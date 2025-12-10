#[cfg(feature = "storage-sd")]
pub(crate) mod sd;

use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::hal::prelude::*;

pub(crate) mod pal {
    pub(super) use embedded_io_async::ErrorType;
    use embedded_io_async::{Read, Seek, Write};

    pub(crate) trait Storage: ErrorType {
        type File: File<Error = Self::Error>;

        async fn read_config<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], Self::Error>;
        async fn write_config(&self, config: &[u8]) -> Result<(), Self::Error>;

        async fn for_each_model<F>(&self, f: F) -> Result<(), Self::Error>
        where
            F: AsyncFnMut(crate::models::Id, &mut Self::File) -> Result<(), Self::Error>;
        async fn model(&self, id: crate::models::Id) -> Result<Option<Self::File>, Self::Error>;
        async fn delete_model(&self, id: crate::models::Id) -> Result<(), Self::Error>;

        async fn flush(&self) -> Result<(), Self::Error>;
    }

    pub(crate) trait File: ErrorType + Read + Write + Seek {
        #[expect(unused)]
        async fn len(&mut self) -> u64;
        #[expect(unused)]
        async fn truncate(&mut self) -> Result<(), Self::Error>;
    }
}

pub(crate) type Error = <crate::hal::Storage as embedded_io_async::ErrorType>::Error;
pub(crate) type File = <crate::hal::Storage as pal::Storage>::File;

type Inner = Mutex<crate::mutex::MultiCore, crate::hal::Storage>;

pub(crate) async fn init(storage: crate::hal::StorageFuture) -> (Manager, Config, Models) {
    static INNER: StaticCell<Inner> = StaticCell::new();

    let storage = storage.await;
    let inner = INNER.init_with(|| Mutex::new(storage));

    (Manager(&*inner), Config(&*inner), Models(&*inner))
}

pub(crate) struct Manager(&'static Inner);

#[derive(Clone, Copy)]
pub(crate) struct Config(&'static Inner);

#[derive(Clone, Copy)]
pub(crate) struct Models(&'static Inner);

impl Manager {
    /// Attempt to flush data before resetting, logging any errors
    pub(crate) async fn flush(self) {
        let storage = self.0.lock().await;

        if let Err(err) = storage.flush().await {
            loog::warn!("Failed to flush: {err:?}");
        }
    }
}

impl Config {
    pub(crate) async fn read(self, buf: &mut [u8]) -> Result<&[u8], Error> {
        let storage = self.0.lock().await;
        storage.read_config(buf).await
    }

    pub(crate) async fn write(self, config: &[u8]) -> Result<(), Error> {
        let storage = self.0.lock().await;
        storage.write_config(config).await
    }
}

impl Models {
    pub(crate) async fn for_each<F>(&self, f: F) -> Result<(), Error>
    where
        F: AsyncFnMut(crate::models::Id, &mut File) -> Result<(), Error>,
    {
        let storage = self.0.lock().await;
        storage.for_each_model(f).await
    }

    pub(crate) async fn model<T>(
        &self,
        id: crate::models::Id,
        mut f: impl AsyncFnMut(&mut File) -> Result<T, Error>,
    ) -> Result<Option<T>, Error> {
        let storage = self.0.lock().await;
        match storage.model(id).await {
            Ok(Some(mut file)) => Ok(Some(f(&mut file).await?)),
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    #[expect(unused)]
    pub(crate) async fn delete(&self, id: crate::models::Id) -> Result<(), Error> {
        let storage = self.0.lock().await;
        storage.delete_model(id).await
    }
}

// #[task]
// pub(crate) async fn run(
//     init: &'static crate::InitCounter,
//     storage: crate::hal::StorageFuture,
//     manager: Manager,
//     config_manager: crate::config::Manager,
//     models: crate::models::Manager,
// ) -> ! {
//     let init = init.start(loog::intern!("storage"));
//
//     let storage = storage.await;
//     let storage = manager.0.get_or_init(|| storage);
//
//     let config = match storage.file("config.bin").await {
//         Ok(file) => file,
//         Err(err) => loog::panic!("Failed to open config file: {err:?}"),
//     };
//     config_manager.load(config).await;
//
//     match root.dir("models").await {
//         Ok(dir) => models.init(dir),
//         Err(err) => loog::panic!("Failed to open models dir: {err:?}"),
//     }
//
//     init.finish();
//     core::future::pending().await
// }
