use std::string::String;
use std::vec::Vec;

use base64::engine::general_purpose::STANDARD_NO_PAD as BASE64;
use base64::Engine as _;

use super::ipc;
use crate::storage::pal;

#[derive(Debug, Clone, Copy)]
pub(super) struct Storage;

#[derive(Debug, Clone)]
pub(super) struct Directory(String);

#[derive(Debug, Clone)]
pub(super) struct File(String);

impl pal::StorageError for Storage {
    type Error = ();
}

impl pal::Storage for Storage {
    type Directory = Directory;

    fn root(&self) -> Self::Directory {
        Directory(String::with_capacity(0))
    }

    async fn flush(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Directory {
    fn push_path(&self, segment: &str) -> String {
        let mut path = self.0.clone();
        path.push('/');
        path.push_str(segment);
        path
    }
}

impl pal::StorageError for Directory {
    type Error = ();
}

impl pal::Directory for Directory {
    type File = File;

    async fn dir(&self, path: &str) -> Result<Self, Self::Error> {
        Ok(Self(self.push_path(path)))
    }

    async fn file(&self, path: &str) -> Result<File, Self::Error> {
        Ok(File(self.push_path(path)))
    }
}

impl pal::StorageError for File {
    type Error = ();
}

impl pal::File for File {
    async fn read_all(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        let data = ipc::storage_read(&self.0).map_or_else(
            || Vec::with_capacity(0),
            |encoded| {
                BASE64
                    .decode(encoded)
                    .expect("valid base64 encoded file data")
            },
        );
        let len = buffer.len().min(data.len());
        buffer[..len].copy_from_slice(&data[..len]);
        Ok(len)
    }

    async fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
        let encoded = BASE64.encode(buffer);
        ipc::storage_write(&self.0, &encoded);
        Ok(())
    }
}
