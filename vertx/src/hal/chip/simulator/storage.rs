use std::convert::Infallible;
use std::string::String;

use embedded_io_async::{ErrorType, Read, Seek, SeekFrom, Write};

use super::ipc;
use crate::storage::pal;

#[derive(Debug, Clone, Copy)]
pub(super) struct Storage;

impl Storage {
    fn path<I: itoa::Integer>(&self, dir: &str, id: I) -> String {
        let mut buffer = itoa::Buffer::new();
        let id = buffer.format(id);

        let mut path = String::from(dir);
        path.push('/');
        path.push_str(id);
        path
    }
}

#[derive(Debug, Clone)]
pub(super) struct File {
    path: String,
    cursor: u64,
}

impl ErrorType for Storage {
    type Error = Infallible;
}

impl pal::Storage for Storage {
    type File = File;

    async fn read_config<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], Self::Error> {
        assert!(buf.len() >= ipc::storage_len("config"));
        let len = ipc::storage_read("config", 0, bytemuck::cast_slice_mut(buf));
        Ok(&buf[0..len])
    }

    async fn write_config(&self, config: &[u8]) -> Result<(), Self::Error> {
        ipc::storage_write("config", 0, bytemuck::cast_slice(config));
        Ok(())
    }

    async fn for_each_model<F>(&self, mut f: F) -> Result<(), Self::Error>
    where
        F: AsyncFnMut(crate::models::Id, &mut Self::File) -> Result<(), Self::Error>,
    {
        for model in ipc::storage_entries("model") {
            let (model, read) = atoi::FromRadix10::from_radix_10(model.as_bytes());
            if read == 0 {
                loog::warn!("Skipping invalid model name: '{model}'");
                continue;
            }

            let path = self.path("model", model);
            f(model, &mut File::new(path)).await?;
        }

        Ok(())
    }

    async fn model(&self, id: crate::models::Id) -> Result<Option<Self::File>, Self::Error> {
        let file = self.path("model", id);

        if ipc::storage_exists(&file) {
            Ok(Some(File::new(file)))
        } else {
            Ok(None)
        }
    }

    async fn delete_model(&self, id: crate::models::Id) -> Result<(), Self::Error> {
        ipc::storage_delete(&self.path("model", id));
        Ok(())
    }

    async fn flush(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl File {
    fn new(path: String) -> Self {
        Self { path, cursor: 0 }
    }
}

impl ErrorType for File {
    type Error = Infallible;
}

impl Seek for File {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.cursor = match pos {
            SeekFrom::Start(x) => x,
            SeekFrom::End(x) => (ipc::storage_len(&self.path) as u64).saturating_add_signed(x),
            SeekFrom::Current(x) => self.cursor.saturating_add_signed(x),
        };
        Ok(self.cursor)
    }
}

impl Read for File {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(ipc::storage_read(&self.path, self.cursor as usize, buf))
    }
}

impl Write for File {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        ipc::storage_write(&self.path, self.cursor as usize, buf);
        Ok(buf.len())
    }
}

impl pal::File for File {
    async fn len(&mut self) -> u64 {
        ipc::storage_len(&self.path) as u64
    }

    async fn truncate(&mut self) -> Result<(), Self::Error> {
        ipc::storage_truncate(&self.path, self.cursor as usize);
        Ok(())
    }
}
