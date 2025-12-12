use core::fmt;

use block_device_driver::BlockDevice;
use embedded_io_async::{ReadExactError, SeekFrom};

use crate::{BLOCK_BYTES, Block, Error, Filesystem, LEN_BYTES};

pub struct File<'buf, 'fs, D: BlockDevice<BLOCK_BYTES>> {
    fs: &'fs mut Filesystem<'buf, D>,
    /// First allocated block
    start: u32,
    /// Final allocated block + 1
    end: u32,
    /// Length of file data in bytes (including file length)
    len: usize,
    /// Current read/write offset in bytes (including file length)
    cursor: usize,
    resized: bool,
    #[cfg(debug_assertions)]
    needs_flush: bool,
}

#[expect(clippy::len_without_is_empty)]
impl<'buf, 'fs, D: BlockDevice<BLOCK_BYTES>> File<'buf, 'fs, D> {
    pub(crate) async fn create(
        fs: &'fs mut Filesystem<'buf, D>,
        start: u32,
        end: u32,
    ) -> Result<Self, Error<D::Error>> {
        loog::trace!("creating file at blocks {start=u32}..={end=u32}");

        let mut view = fs.buffer.select(&mut fs.device, start).await?;
        crate::write_len(view.data_mut(), 0);
        view.mark_modified(0, LEN_BYTES);

        Ok(Self {
            fs,
            start,
            end,
            len: LEN_BYTES,
            cursor: LEN_BYTES,
            resized: false,
            #[cfg(debug_assertions)]
            needs_flush: false,
        })
    }

    pub(crate) async fn open(
        fs: &'fs mut Filesystem<'buf, D>,
        start: u32,
        end: u32,
    ) -> Result<Self, Error<D::Error>> {
        let mut view = fs.buffer.select(&mut fs.device, start).await?;
        view.read().await?;
        let len = crate::read_len(view.data());
        loog::trace!("opening {len} byte file at blocks {start=u32}..={end=u32}");

        Ok(Self {
            fs,
            start,
            end,
            len: len + LEN_BYTES,
            cursor: LEN_BYTES,
            resized: false,
            #[cfg(debug_assertions)]
            needs_flush: false,
        })
    }

    pub fn len(&mut self) -> u64 {
        (self.len - LEN_BYTES) as u64
    }

    fn remaining(&self) -> usize {
        self.len - self.cursor
    }

    pub fn truncate(&mut self) {
        if self.cursor < self.len {
            self.resized = true;
            self.len = self.cursor;
        }
    }

    fn block(&self) -> u32 {
        self.start + (self.cursor / BLOCK_BYTES) as u32
    }
}

impl<D: BlockDevice<BLOCK_BYTES>> File<'_, '_, D>
where
    D::Error: embedded_io_async::Error,
{
    pub async fn close(mut self) -> Result<(), Error<D::Error>> {
        embedded_io_async::Write::flush(&mut self).await?;
        Ok(())
    }
}

impl<D: BlockDevice<BLOCK_BYTES>> embedded_io_async::ErrorType for File<'_, '_, D>
where
    D::Error: embedded_io_async::Error,
{
    type Error = Error<D::Error>;
}

impl<D: BlockDevice<BLOCK_BYTES>> embedded_io_async::Seek for File<'_, '_, D>
where
    D::Error: embedded_io_async::Error,
{
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        let target = match pos {
            SeekFrom::Start(offset) => Some(LEN_BYTES + offset as usize),
            SeekFrom::End(offset) => self.len.checked_add_signed(offset as isize),
            SeekFrom::Current(offset) => self.cursor.checked_add_signed(offset as isize),
        };

        if let Some(new) = target
            && (LEN_BYTES <= new && new <= self.len)
        {
            self.cursor = new;
            Ok((new - LEN_BYTES) as u64)
        } else {
            Err(Error::SeekOutOfBounds)
        }
    }

    async fn stream_position(&mut self) -> Result<u64, Self::Error> {
        Ok((self.cursor - LEN_BYTES) as u64)
    }
}

impl<D: BlockDevice<BLOCK_BYTES>> embedded_io_async::Read for File<'_, '_, D>
where
    D::Error: embedded_io_async::Error,
{
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        loog::trace!(
            "reading up to {} bytes from file at block {=u32}",
            buf.len(),
            self.block()
        );

        let len = self.remaining().min(buf.len());

        let start = self.block();
        let mut view = self.fs.buffer.select(&mut self.fs.device, start).await?;
        view.read().await?;
        let data = Block::as_byte_slice(view.data());

        let offset = self.cursor % BLOCK_BYTES;
        let len = len.min(data.len() - offset);

        buf[0..len].copy_from_slice(&data[offset..(offset + len)]);
        self.cursor += len;
        Ok(len)
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), ReadExactError<Self::Error>> {
        loog::trace!(
            "reading exactly {} bytes from file at block {=u32}",
            buf.len(),
            self.block()
        );

        let io_err = |err| ReadExactError::Other(Error::Io(err));

        let end = self.cursor + buf.len();
        if end > self.len {
            return Err(ReadExactError::UnexpectedEof);
        }

        let start = self.block();
        let end = (self.cursor + buf.len()).div_ceil(BLOCK_BYTES) as u32;
        let len_blocks = end - start;
        if len_blocks > self.fs.buffer.len() {
            return Err(ReadExactError::UnexpectedEof);
        }

        let mut view = self
            .fs
            .buffer
            .select_exact(&mut self.fs.device, start, len_blocks)
            .await
            .map_err(io_err)?;
        view.read().await.map_err(io_err)?;
        let data = Block::as_byte_slice(view.data());

        let offset = self.cursor % BLOCK_BYTES;
        buf.copy_from_slice(&data[offset..(offset + buf.len())]);
        self.cursor += buf.len();
        Ok(())
    }
}

impl<D: BlockDevice<BLOCK_BYTES>> embedded_io_async::Write for File<'_, '_, D>
where
    D::Error: embedded_io_async::Error,
{
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        loog::trace!(
            "writing up to {} bytes to file at block {=u32}",
            buf.len(),
            self.block()
        );

        #[cfg(debug_assertions)]
        {
            self.needs_flush = true;
        }

        if buf.len() > self.remaining() {
            let new_len = self.cursor + buf.len();
            let new_end = new_len.div_ceil(BLOCK_BYTES) as u32;
            if new_end > self.end {
                return Err(Error::FileFull);
            }

            loog::trace!("growing from {} to {} bytes", self.len, new_len);

            self.len = new_len;
            self.resized = true;
        }

        let start = self.block();
        let offset = self.cursor % BLOCK_BYTES;

        let mut view = self.fs.buffer.select(&mut self.fs.device, start).await?;
        if offset > 0 || buf.len() < BLOCK_BYTES {
            view.read().await?;
        }
        let data = Block::as_byte_slice_mut(view.data_mut());

        let len = buf.len().min(data.len() - offset);
        data[offset..(offset + len)].copy_from_slice(&buf[0..len]);
        self.cursor += len;
        Ok(len)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        loog::trace!("flushing file at block {=u32}", self.block());

        if self.resized {
            loog::trace!("file has been resized; updating len");

            let mut view = self
                .fs
                .buffer
                .select(&mut self.fs.device, self.start)
                .await?;
            view.read().await?;
            crate::write_len(view.data_mut(), self.len - LEN_BYTES);
            view.mark_modified(0, LEN_BYTES);
        }

        self.fs.flush().await?;

        #[cfg(debug_assertions)]
        {
            self.needs_flush = false;
        }

        Ok(())
    }
}

#[cfg(debug_assertions)]
impl<D: BlockDevice<BLOCK_BYTES>> Drop for File<'_, '_, D> {
    fn drop(&mut self) {
        if self.needs_flush {
            loog::panic!("file dropped without flushing");
        }
    }
}

impl<D: BlockDevice<BLOCK_BYTES>> fmt::Debug for File<'_, '_, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File")
            .field("start", &self.start)
            .field("end", &self.end)
            .field("len", &self.len)
            .field("cursor", &self.cursor)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::array;

    use embedded_io_async::{Read as _, Seek as _, Write as _};

    use super::*;
    use crate::Mock;

    #[tokio::test]
    #[test_log::test]
    async fn create() {
        let mut mock = Mock::<2>::new();
        let mut buffers = crate::Buffers::new();
        let mut fs = Filesystem::new_empty(&mut mock, &mut buffers);

        let mut file = File::create(&mut fs, 1, 2).await.unwrap();
        assert_eq!(file.len(), 0);
        assert_eq!(
            file.seek(SeekFrom::Start(1)).await.unwrap_err(),
            Error::SeekOutOfBounds
        );
        assert_eq!(file.cursor, LEN_BYTES);
        file.write_all(&[1, 2]).await.unwrap();
        assert_eq!(file.len(), 2);
        file.close().await.unwrap();

        let blocks = mock.blocks();
        assert_eq!(blocks[0].iter().find(|x| **x != 0), None);
        assert_eq!(&blocks[1][0..8], &[2, 0, 0, 0, 1, 2, 0, 0]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn open() {
        let mut mock = Mock::<1>::new();
        {
            let mock = mock.block_mut(0);
            mock[0] = 9;
            mock[4..13].copy_from_slice(&array::from_fn::<_, 9, _>(|x| x as u8 + 1)[..]);
        }

        let mut buffers = crate::Buffers::new();
        let mut fs = Filesystem::new_empty(&mut mock, &mut buffers);

        let mut file = File::open(&mut fs, 0, 1).await.unwrap();
        assert_eq!(file.len(), 9);
        let mut buf = [0; 9];
        assert_eq!(file.read(&mut buf).await.unwrap(), buf.len());
        file.rewind().await.unwrap();
        file.truncate();
        file.write(&[buf.iter().sum()]).await.unwrap();
        file.close().await.unwrap();

        let mut file = File::open(&mut fs, 0, 1).await.unwrap();
        assert_eq!(file.len(), 1);
        let mut buf = [0];
        assert_eq!(file.read(&mut buf).await.unwrap(), 1);
        assert_eq!(buf, [45]);
    }
}
