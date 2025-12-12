#![expect(unused_variables)]
#![no_std]

#[cfg(test)]
extern crate std;

mod block;
mod buffer;
mod file;
mod header;
#[cfg(test)]
mod mock;

use core::fmt;

use aligned::Alignment;
use block_device_driver::BlockDevice;
#[cfg(feature = "defmt")]
use loog::defmt;

pub(crate) use self::block::Block;
pub(crate) use self::buffer::Buffer;
pub use self::file::File;
pub use self::header::Error as HeaderError;
use self::header::Header;
#[cfg(test)]
pub(crate) use self::mock::Mock;

pub const BLOCK_BYTES: usize = 512;
/// Number of bytes used to store file lengths, etc
pub(crate) const LEN_BYTES: usize = 4;

const HEADER_BLOCK: u32 = 0;
const NAMES_OFFSET: u32 = 1;
const CONFIG_BLOCK: u32 = 3;
const MODELS_START: u32 = 4;

/// Fixed number of blocks to allocate for each model file
pub(crate) const MODEL_BLOCKS: u32 = 4;
pub(crate) const MODEL_NAME_BYTES: usize = 16;
/// Chosen to fit 64 * 16 byte model names in blocks 1 & 2
pub(crate) const MAX_MODELS: usize = 64;

#[derive(Debug, Clone, PartialEq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error<I> {
    SeekOutOfBounds,
    /// The operation would require allocating more backing blocks for the file,
    /// which is (yet) supported.
    FileFull,
    TooManyModels,
    ModelNameOverflow,
    Io(I),
}

#[allow(clippy::match_same_arms)]
impl<I: fmt::Debug + embedded_io_async::Error> embedded_io_async::Error for Error<I> {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        use embedded_io_async::ErrorKind;
        match self {
            Self::SeekOutOfBounds => ErrorKind::InvalidInput,
            Self::FileFull => ErrorKind::Unsupported,
            Self::TooManyModels => ErrorKind::InvalidData,
            Self::ModelNameOverflow => ErrorKind::InvalidInput,
            Self::Io(err) => err.kind(),
        }
    }
}

impl<I> From<I> for Error<I> {
    fn from(io: I) -> Self {
        Self::Io(io)
    }
}

pub enum InitError<'buf, D: BlockDevice<BLOCK_BYTES>> {
    HeaderError {
        kind: HeaderError,
        device: D,
        buffers: &'buf mut Buffers<D::Align>,
    },
    Io(D::Error),
}

pub struct Filesystem<'buf, D: BlockDevice<BLOCK_BYTES>> {
    device: D,
    header: &'buf mut Block<D::Align>,
    buffer: Buffer<'buf, D::Align, 2>,
}

pub struct Buffers<A> {
    header: Block<A>,
    buffer: [Block<A>; 2],
}

impl<A: aligned::Alignment> Buffers<A> {
    pub const fn new() -> Self {
        Self {
            header: Block::new(),
            buffer: [const { Block::new() }; 2],
        }
    }
}

impl<A: aligned::Alignment> Default for Buffers<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'buf, D: BlockDevice<BLOCK_BYTES>> Filesystem<'buf, D> {
    pub async fn new(
        mut device: D,
        buffers: &'buf mut Buffers<D::Align>,
    ) -> Result<Self, InitError<'buf, D>> {
        device
            .read(HEADER_BLOCK, buffers.header.as_aligned_mut())
            .await
            .map_err(InitError::Io)?;

        let header = Header::from_block(&buffers.header);
        if let Err(error) = header.validate() {
            return Err(InitError::HeaderError {
                kind: error,
                device,
                buffers,
            });
        }

        let Buffers { header, buffer } = buffers;
        Ok(Self {
            device,
            header,
            buffer: Buffer::new(buffer),
        })
    }

    pub fn new_empty(device: D, buffers: &'buf mut Buffers<D::Align>) -> Self {
        let header = Header::from_block_mut(&mut buffers.header);
        header.init();

        let Buffers { header, buffer } = buffers;
        Self {
            device,
            header,
            buffer: Buffer::new(buffer),
        }
    }

    pub async fn read_config<'a>(
        &mut self,
        buf: &'a mut [u8],
    ) -> Result<&'a [u8], Error<D::Error>> {
        debug_assert!(buf.len() <= (BLOCK_BYTES - LEN_BYTES));

        let mut view = self.buffer.select(&mut self.device, CONFIG_BLOCK).await?;
        view.read().await?;
        let data = read_slice(view.data());

        let out = &mut buf[0..data.len()];
        out.copy_from_slice(data);
        Ok(out)
    }

    pub async fn write_config(&mut self, config: &[u8]) -> Result<(), Error<D::Error>> {
        debug_assert!(config.len() <= (BLOCK_BYTES - LEN_BYTES));

        let total_len = config.len() + LEN_BYTES;
        let mut view = self.buffer.select(&mut self.device, CONFIG_BLOCK).await?;
        write_slice(view.data_mut(), config);
        view.mark_modified(0, total_len);

        Ok(())
    }

    pub async fn model_names<F>(&mut self, f: F) -> Result<(), Error<D::Error>>
    where
        F: FnMut(u8, &str),
    {
        let mut view = self
            .buffer
            .select_exact(&mut self.device, NAMES_OFFSET, 2)
            .await?;
        view.read().await?;
        let names = Block::as_byte_slice(view.data());

        for model in Header::from_block(self.header).iter_models() {
            if model.start() == 0 {
                break;
            }

            let offset = usize::from(model.id()) * MODEL_NAME_BYTES;
            let name = &names[offset..(offset + MODEL_NAME_BYTES)];
            let len = name
                .iter()
                .take_while(|x| (b' '..=b'~').contains(*x))
                .count();
            // SAFETY: name[0..len] is validated to be a subset of ascii, so must be valid
            // UTF-8, too
            let name = unsafe { str::from_utf8_unchecked(&name[0..len]) };
        }

        Ok(())
    }

    pub async fn model<'fs>(
        &'fs mut self,
        id: u8,
    ) -> Result<Option<File<'buf, 'fs, D>>, Error<D::Error>> {
        let header = Header::from_block(self.header);
        let Some(model) = header.iter_models().find(|model| model.id() == id) else {
            return Ok(None);
        };

        debug_assert!(model.start() >= MODELS_START);

        let file = File::open(self, model.start(), model.end()).await?;
        Ok(Some(file))
    }

    pub async fn new_model<'fs>(
        &'fs mut self,
        name: &str,
    ) -> Result<File<'buf, 'fs, D>, Error<D::Error>> {
        loog::trace!("creating new model: {name=str:?}");

        let name = name.as_bytes();
        if name.len() > MODEL_NAME_BYTES {
            return Err(Error::ModelNameOverflow);
        }

        let header = Header::from_block_mut(self.header);
        let Some(model) = header.new_model() else {
            return Err(Error::TooManyModels);
        };

        loog::trace!(
            "allocated new model with id {=u8} at blocks {=u32}..={=u32}",
            model.id(),
            model.start(),
            model.end()
        );

        let mut view = self
            .buffer
            .select_exact(&mut self.device, NAMES_OFFSET, 2)
            .await?;
        view.read().await?;
        let names = Block::as_byte_slice_mut(view.data_mut());
        let name_start = usize::from(model.id()) * MODEL_NAME_BYTES;
        let name_end = name_start + name.len();
        names[name_start..name_end].copy_from_slice(name);
        names[name_end..].fill(0);

        let start = model.start();
        let end = model.end();
        let file = File::create(self, start, end).await?;
        Ok(file)
    }

    pub async fn delete_model(&mut self, id: u8) -> Result<(), Error<D::Error>> {
        let header = Header::from_block_mut(self.header);
        header.delete_model(id);
        self.write_header().await
    }

    pub async fn flush(&mut self) -> Result<(), Error<D::Error>> {
        self.buffer.flush(&mut self.device).await?;
        Ok(())
    }

    async fn write_header(&mut self) -> Result<(), Error<D::Error>> {
        self.device
            .write(HEADER_BLOCK, self.header.as_aligned())
            .await?;
        Ok(())
    }
}

impl<D: BlockDevice<BLOCK_BYTES>> embedded_io_async::ErrorType for Filesystem<'_, D>
where
    D::Error: embedded_io_async::Error,
{
    type Error = Error<D::Error>;
}

impl<D: BlockDevice<BLOCK_BYTES>> fmt::Debug for InitError<'_, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderError { kind: error, .. } => f
                .debug_struct("HeaderError")
                .field("kind", error)
                .finish_non_exhaustive(),
            Self::Io(io) => f.debug_tuple("Io").field(io).finish(),
        }
    }
}

impl<A> fmt::Debug for Buffers<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Buffers").finish_non_exhaustive()
    }
}

impl<D: BlockDevice<BLOCK_BYTES>> fmt::Debug for Filesystem<'_, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Filesystem").finish_non_exhaustive()
    }
}

fn read_len<A: Alignment>(buffer: &[Block<A>]) -> usize {
    u32::from_le(Block::as_word_slice(buffer)[0]) as usize
}

fn read_slice<A: Alignment>(buffer: &[Block<A>]) -> &[u8] {
    let len = read_len(buffer);
    &Block::as_byte_slice(buffer)[LEN_BYTES..(LEN_BYTES + len)]
}

fn write_len<A: Alignment>(buffer: &mut [Block<A>], len: usize) {
    Block::as_word_slice_mut(buffer)[0] = (len as u32).to_le();
}

fn write_slice<A: Alignment>(buffer: &mut [Block<A>], data: &[u8]) {
    write_len(buffer, data.len());
    Block::as_byte_slice_mut(buffer)[LEN_BYTES..(data.len() + LEN_BYTES)].copy_from_slice(data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[test_log::test]
    async fn model_name() {
        let mut mock = Mock::<5>::new();
        let mut buffers = crate::Buffers::new();
        let mut fs = Filesystem::new_empty(&mut mock, &mut buffers);

        let mut file = fs.new_model("Test model").await.unwrap();
        assert_eq!(file.len(), 0);
        file.close().await.unwrap();
    }
}
