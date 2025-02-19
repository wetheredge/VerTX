use std::cmp::Ordering;
use std::string::String;
use std::vec::{self, Vec};

use embedded_io_async::{ErrorType, Read, Seek, SeekFrom, Write};

use super::ipc;
use crate::storage::pal;

#[derive(Debug, Clone, Copy)]
pub(super) struct Storage;

#[derive(Debug, Clone)]
pub(super) struct Directory(String);

#[derive(Debug, Clone)]
pub(super) struct File {
    path: String,
    cursor: u64,
}

#[derive(Debug, Clone)]
pub(super) struct DirectoryIter(vec::IntoIter<Entry>);

#[derive(Debug, Clone)]
pub(super) struct Entry {
    path: String,
    is_file: bool,
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.path.cmp(&other.path))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for Entry {}

#[derive(Debug, Clone, Copy)]
pub(super) enum NeverError {}

impl embedded_io_async::Error for NeverError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        match *self {}
    }
}

impl ErrorType for Storage {
    type Error = NeverError;
}

impl pal::Storage for Storage {
    type Directory = Directory;

    const FILENAME_BYTES: usize = 12;

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

impl ErrorType for Directory {
    type Error = NeverError;
}

impl pal::Directory for Directory {
    type File = File;
    type Iter = DirectoryIter;

    async fn dir(&self, path: &str) -> Result<Self, Self::Error> {
        Ok(Self(self.push_path(path)))
    }

    async fn file(&self, path: &str) -> Result<File, Self::Error> {
        Ok(File::new(self.push_path(path)))
    }

    fn iter(&self) -> Self::Iter {
        DirectoryIter::new(&self.0)
    }
}

impl File {
    fn new(path: String) -> Self {
        Self { path, cursor: 0 }
    }
}

impl ErrorType for File {
    type Error = NeverError;
}

impl Seek for File {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.cursor = match pos {
            SeekFrom::Start(x) => x,
            SeekFrom::End(x) => (ipc::storage_file_len(&self.path) as u64).saturating_add_signed(x),
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
    async fn truncate(&mut self) -> Result<(), Self::Error> {
        ipc::storage_truncate(&self.path, self.cursor as usize);
        Ok(())
    }

    async fn close(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl DirectoryIter {
    fn new(root: &str) -> Self {
        let root = {
            let mut s = String::from(root);
            s.push('/');
            s
        };

        let mut entries = ipc::storage_dir_entries(&root)
            .into_iter()
            .map(|mut path| {
                if let Some((dir, _)) = path.split_once('/') {
                    let mut path = root.clone();
                    path.push_str(dir);
                    path.push('/');
                    Entry {
                        path,
                        is_file: false,
                    }
                } else {
                    path.insert_str(0, &root);
                    Entry {
                        path,
                        is_file: true,
                    }
                }
            })
            .collect::<Vec<_>>();
        entries.sort_unstable();
        entries.dedup();

        Self(entries.into_iter())
    }
}

impl ErrorType for DirectoryIter {
    type Error = NeverError;
}

impl pal::DirectoryIter for DirectoryIter {
    type Directory = Directory;
    type Entry = Entry;
    type File = File;

    async fn next(&mut self) -> Option<Result<Self::Entry, Self::Error>> {
        self.0.next().map(Ok)
    }
}

impl ErrorType for Entry {
    type Error = NeverError;
}

impl pal::Entry for Entry {
    type Directory = Directory;
    type File = File;

    fn name(&self) -> &[u8] {
        self.path.rsplit_once('/').unwrap().1.as_bytes()
    }

    fn is_file(&self) -> bool {
        self.is_file
    }

    fn is_dir(&self) -> bool {
        !self.is_file
    }

    fn to_file(self) -> Option<Self::File> {
        self.is_file().then(|| File::new(self.path))
    }

    fn to_dir(self) -> Option<Self::Directory> {
        self.is_dir().then_some(Directory(self.path))
    }
}
