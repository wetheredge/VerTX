use core::error::Error as CoreError;
use core::fmt;

use block_device_driver::BlockDevice;
use embedded_io_async as eio;
#[cfg(feature = "defmt")]
use loog::defmt;

use crate::BLOCK_BYTES;

#[derive(Debug, Clone, PartialEq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error<I> {
    SeekOutOfBounds,
    /// The operation would require allocating more backing blocks for the
    /// file, which is (yet) supported.
    FileFull,
    TooManyModels,
    ModelNameOverflow,
    Io(I),
}

impl<I: fmt::Display> fmt::Display for Error<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SeekOutOfBounds => f.write_str("seek out of bounds"),
            Self::FileFull => f.write_str("file is full and cannot be reallocated"),
            Self::TooManyModels => f.write_str("no model ids left"),
            Self::ModelNameOverflow => f.write_str("model name is too long"),
            Self::Io(io) => fmt::Display::fmt(io, f),
        }
    }
}

impl<I: CoreError> CoreError for Error<I> {}

#[allow(clippy::match_same_arms)]
impl<I: eio::Error> eio::Error for Error<I> {
    fn kind(&self) -> eio::ErrorKind {
        match self {
            Self::SeekOutOfBounds => eio::ErrorKind::InvalidInput,
            Self::FileFull => eio::ErrorKind::Unsupported,
            Self::TooManyModels => eio::ErrorKind::InvalidData,
            Self::ModelNameOverflow => eio::ErrorKind::InvalidInput,
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
        kind: crate::HeaderError,
        device: D,
        buffers: &'buf mut crate::Buffers<D::Align>,
    },
    Io(D::Error),
}

impl<D: BlockDevice<BLOCK_BYTES>> fmt::Debug for InitError<'_, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderError { kind, .. } => f
                .debug_struct("HeaderError")
                .field("kind", kind)
                .finish_non_exhaustive(),
            Self::Io(io) => f.debug_tuple("Io").field(io).finish(),
        }
    }
}
