#[cfg(feature = "storage-sd")]
pub(crate) mod sd;

use embassy_executor::task;
use embassy_sync::once_lock::OnceLock;

use crate::hal::prelude::*;

pub(crate) mod pal {
    pub(super) use embedded_io_async::ErrorType;
    use embedded_io_async::{Read, Seek, Write};

    pub(crate) trait Storage: ErrorType {
        type Directory: Directory<Error = Self::Error>;

        fn root(&self) -> Self::Directory;
        async fn flush(&self) -> Result<(), Self::Error>;
    }

    pub(crate) trait Directory: Clone + ErrorType {
        type File: File<Error = Self::Error>;
        type Iter: DirectoryIter<Error = Self::Error, File = Self::File>;

        async fn dir(&self, path: &str) -> Result<Self, Self::Error>;
        async fn file(&self, path: &str) -> Result<Self::File, Self::Error>;
        fn iter(&self) -> Self::Iter;
    }

    pub(crate) trait File: Clone + ErrorType + Read + Write + Seek {
        async fn truncate(&mut self) -> Result<(), Self::Error>;
        async fn close(self) -> Result<(), Self::Error>;
    }

    pub(crate) trait DirectoryIter: Clone + ErrorType {
        type File: File<Error = Self::Error>;
        type Directory: Directory<Error = Self::Error>;
        type Entry: Entry<Error = Self::Error, File = Self::File>;

        async fn next(&mut self) -> Option<Result<Self::Entry, Self::Error>>;
    }

    pub(crate) trait Entry: ErrorType {
        type File: File<Error = Self::Error>;
        type Directory: Directory<Error = Self::Error>;

        fn name(&self) -> &[u8];
        fn is_file(&self) -> bool;
        fn to_file(self) -> Option<Self::File>;
        #[expect(unused)]
        fn is_dir(&self) -> bool;
        #[expect(unused)]
        fn to_dir(self) -> Option<Self::Directory>;
    }
}

pub(crate) type Error = <crate::hal::Storage as embedded_io_async::ErrorType>::Error;
pub(crate) type Directory = <crate::hal::Storage as pal::Storage>::Directory;
pub(crate) type File = <Directory as pal::Directory>::File;

#[derive(Clone, Copy)]
pub(crate) struct Manager(&'static OnceLock<crate::hal::Storage>);

impl Manager {
    pub(crate) const fn new() -> Self {
        static INNER: OnceLock<crate::hal::Storage> = OnceLock::new();
        Self(&INNER)
    }

    /// Attempt to flush data before resetting, logging any errors
    pub(crate) async fn flush_before_reset(self) {
        let Some(storage) = self.0.try_get() else {
            loog::warn!("Skipping flush since it was not initialized");
            return;
        };

        if let Err(err) = storage.flush().await {
            loog::warn!("Failed to flush: {err:?}");
        }
    }
}

#[task]
pub(crate) async fn run(
    init: &'static crate::InitCounter,
    storage: crate::hal::StorageFuture,
    manager: Manager,
    config_manager: crate::config::Manager,
    models: crate::models::Manager,
) -> ! {
    let init = init.start(loog::intern!("storage"));

    let storage = storage.await;
    let storage = manager.0.get_or_init(|| storage);
    let root = storage.root();

    let config = match root.file("config.bin").await {
        Ok(file) => file,
        Err(err) => loog::panic!("Failed to open config file: {err:?}"),
    };
    config_manager.load(config).await;

    match root.dir("models").await {
        Ok(dir) => models.init(dir),
        Err(err) => loog::panic!("Failed to open models dir: {err:?}"),
    }

    init.finish();
    core::future::pending().await
}
