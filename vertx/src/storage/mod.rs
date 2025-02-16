#[allow(unused)]
pub(crate) mod sd;

use embassy_executor::task;

use crate::hal::prelude::*;

pub(crate) mod pal {
    use embedded_io_async::ReadExactError;

    pub(crate) trait Storage {
        type Error: loog::DebugFormat;
        type Directory: Directory<Error = Self::Error>;

        const FILENAME_BYTES: usize;

        fn root(&self) -> Self::Directory;
        async fn flush(&self) -> Result<(), Self::Error>;
    }

    pub(crate) trait Directory: Clone {
        type Error: loog::DebugFormat;
        type File: File<Error = Self::Error>;
        type Iter: DirectoryIter<Error = Self::Error, File = Self::File>;

        async fn dir(&self, path: &str) -> Result<Self, Self::Error>;
        async fn file(&self, path: &str) -> Result<Self::File, Self::Error>;
        fn iter(&self) -> Self::Iter;
    }

    pub(crate) trait File: Clone {
        type Error: loog::DebugFormat;

        async fn seek_to_start(&mut self) -> Result<(), Self::Error>;
        async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
        async fn flush(&mut self) -> Result<(), Self::Error>;

        async fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error>;

        async fn read_exact(
            &mut self,
            buffer: &mut [u8],
        ) -> Result<(), ReadExactError<Self::Error>> {
            let mut len = 0;
            while len < buffer.len() {
                let chunk = self.read(&mut buffer[len..]).await?;
                len += chunk;
                if chunk == 0 {
                    return Err(ReadExactError::UnexpectedEof);
                }
            }

            Ok(())
        }

        async fn read_all(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
            self.seek_to_start().await?;

            let mut len = 0;
            while len < buffer.len() {
                let chunk = self.read(&mut buffer[len..]).await?;
                len += chunk;
                if chunk == 0 {
                    break;
                }
            }

            Ok(len)
        }
    }

    pub(crate) trait DirectoryIter: Clone {
        type Error: loog::DebugFormat;
        type File: File<Error = Self::Error>;
        type Directory: Directory<Error = Self::Error>;
        type Entry: Entry<Error = Self::Error, File = Self::File>;

        async fn next(&mut self) -> Option<Result<Self::Entry, Self::Error>>;
    }

    pub(crate) trait Entry {
        type Error: loog::DebugFormat;
        type File: File<Error = Self::Error>;
        type Directory: Directory<Error = Self::Error>;

        fn name(&self) -> &[u8];
        fn to_file(self) -> Option<Self::File>;
        #[expect(unused)]
        fn to_dir(self) -> Option<Self::Directory>;
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

    let mut models = match root.dir("models").await {
        Ok(models) => models.iter(),
        Err(err) => loog::panic!("Failed to open models dir: {err:?}"),
    };
    while let Some(entry) = models.next().await {
        let entry = loog::unwrap!(entry);
        let mut filename = [0; crate::hal::Storage::FILENAME_BYTES];
        let filename = {
            let bytes = entry.name();
            let filename = &mut filename[0..bytes.len()];
            filename.copy_from_slice(bytes);
            &*filename
        };

        let Some(mut file) = entry.to_file() else {
            continue;
        };
        let mut name = [0; 16];
        let name = {
            let mut len = [0; 1];
            loog::unwrap!(file.read(&mut len).await);
            let [len] = len;
            let len = len as usize;

            if len > name.len() {
                loog::panic!("Invalid name length: {len} > {}", (name.len()));
            }

            let name = &mut name[0..len];
            loog::unwrap!(file.read_exact(name).await);
            &*name
        };
        loog::debug!("{filename}: {:?}", core::str::from_utf8(name).unwrap(),);
    }
    loog::debug!("Listed all models");

    init.finish();
    core::future::pending().await
}
