pub(crate) mod sd;

use embassy_executor::task;

use crate::hal::prelude::*;

pub(crate) mod pal {
    pub(crate) trait StorageError {
        type Error: loog::DebugFormat;
    }

    pub(crate) trait Storage: StorageError {
        type Directory: Directory<Error = Self::Error>;

        fn root(&self) -> Self::Directory;
        async fn flush(&self) -> Result<(), Self::Error>;
    }

    pub(crate) trait Directory: StorageError + Sized {
        type File: File<Error = Self::Error>;

        async fn dir(&self, path: &str) -> Result<Self, Self::Error>;
        async fn file(&self, path: &str) -> Result<Self::File, Self::Error>;
    }

    pub(crate) trait File: StorageError + Clone {
        async fn read_all(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
        async fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error>;
    }
}

pub(crate) type Directory = <crate::hal::Storage as pal::Storage>::Directory;
pub(crate) type File = <Directory as pal::Directory>::File;

#[task]
pub(crate) async fn run(
    init: &'static crate::InitCounter,
    storage: crate::hal::StorageFuture,
    config_manager: &'static crate::config::Manager,
) -> ! {
    let init = init.start(loog::intern!("storage"));

    let storage = storage.await;
    let root = storage.root();

    let config = match root.file("config.bin").await {
        Ok(file) => file,
        Err(err) => loog::panic!("Failed to open config file: {err:?}"),
    };
    config_manager.load(config).await;

    init.finish();
    core::future::pending().await
}
