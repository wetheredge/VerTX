use std::convert::Infallible;
use std::string::String;
use std::vec::Vec;

use embedded_io_async::{ErrorType, Read, ReadExactError, Seek, SeekFrom, Write};

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

impl ErrorType for Storage {
    type Error = Infallible;
}

impl pal::Storage for Storage {
    type File<'s>
        = File
    where
        Self: 's;

    async fn read_config<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8], Self::Error> {
        let read = ipc::fs_read("config");
        loog::debug!("config exists: {}", read.is_some());
        let read = read.as_deref().unwrap_or_default();

        let buf = &mut buf[0..read.len()];
        buf.copy_from_slice(read);
        Ok(buf)
    }

    async fn write_config(&mut self, config: &[u8]) -> Result<(), Self::Error> {
        ipc::fs_write("config", config);
        Ok(())
    }

    async fn model_names<F>(&mut self, mut f: F) -> Result<(), Self::Error>
    where
        F: FnMut(crate::models::Id, &str),
    {
        for model_str in ipc::fs_list("model/name/") {
            let (model, len) = atoi::FromRadix10::from_radix_10(model_str.as_bytes());
            if len == 0 || len != model_str.len() {
                loog::warn!("Skipping invalid model name: '{model_str}'");
                continue;
            }

            let mut path = String::from("model/name/");
            path.push_str(&model_str);

            let name = ipc::fs_read(&path).unwrap();
            let name = String::from_utf8(name).expect("model name is valid UTF-8");
            f(model, &name);
        }

        Ok(())
    }

    async fn model(
        &mut self,
        id: crate::models::Id,
    ) -> Result<Option<Self::File<'_>>, Self::Error> {
        let path = self.path("model/data", id);
        Ok(File::open(path))
    }

    async fn delete_model(&mut self, id: crate::models::Id) -> Result<(), Self::Error> {
        ipc::fs_delete(&self.path("model/data", id));
        ipc::fs_delete(&self.path("model/name", id));
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct File {
    path: String,
    data: Vec<u8>,
    cursor: usize,
}

impl File {
    fn open(path: String) -> Option<Self> {
        let data = ipc::fs_read(&path)?;
        Some(Self {
            path,
            cursor: 0,
            data,
        })
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.cursor
    }
}

impl ErrorType for File {
    type Error = Infallible;
}

impl Seek for File {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.cursor = match pos {
            SeekFrom::Start(x) => x as usize,
            SeekFrom::End(x) => self.data.len().saturating_add_signed(x as isize),
            SeekFrom::Current(x) => self.cursor.saturating_add_signed(x as isize),
        };
        Ok(self.cursor as u64)
    }
}

impl Read for File {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let len = buf.len().min(self.remaining());
        buf[0..len].copy_from_slice(&self.data[self.cursor..(self.cursor + len)]);
        self.cursor += len;
        Ok(len)
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), ReadExactError<Self::Error>> {
        if self.remaining() < buf.len() {
            return Err(ReadExactError::UnexpectedEof);
        }

        let Ok(_) = self.read(buf).await;
        Ok(())
    }
}

impl Write for File {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let overwrite = buf.len().min(self.remaining());
        self.data[self.cursor..(self.cursor + overwrite)].copy_from_slice(&buf[0..overwrite]);
        self.data.extend_from_slice(&buf[overwrite..]);
        self.cursor += buf.len();
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        ipc::fs_write(&self.path, &self.data);
        Ok(())
    }
}

impl pal::File for File {
    async fn len(&mut self) -> u64 {
        self.data.len() as u64
    }

    async fn truncate(&mut self) -> Result<(), Self::Error> {
        self.data.truncate(self.cursor);
        Ok(())
    }
}
