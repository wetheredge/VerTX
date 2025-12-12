use std::fmt;

use aligned::Aligned;
use block_device_driver::BlockDevice;
use embedded_io_async::ErrorKind;

use crate::BLOCK_BYTES;

#[derive(Debug)]
pub(crate) struct Mock<const LEN: usize>([[u8; BLOCK_BYTES]; LEN]);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MockError {
    OutOfBounds,
}

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds => f.write_str("out of bounds"),
        }
    }
}

impl std::error::Error for MockError {}

impl embedded_io_async::Error for MockError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::OutOfBounds => ErrorKind::OutOfMemory,
        }
    }
}

impl<const LEN: usize> Mock<LEN> {
    pub(crate) const fn new() -> Self {
        Self([[0; BLOCK_BYTES]; LEN])
    }

    pub(crate) fn blocks(&self) -> &[[u8; BLOCK_BYTES]; LEN] {
        &self.0
    }

    pub(crate) fn block_mut(&mut self, i: usize) -> &mut [u8; BLOCK_BYTES] {
        &mut self.0[i]
    }
}

impl<const LEN: usize> BlockDevice<BLOCK_BYTES> for Mock<LEN> {
    type Align = aligned::A1;
    type Error = MockError;

    async fn read(
        &mut self,
        block_address: u32,
        data: &mut [Aligned<Self::Align, [u8; BLOCK_BYTES]>],
    ) -> Result<(), Self::Error> {
        loog::debug!(
            "reading {} block(s), starting at {block_address}",
            data.len()
        );

        let mut data = data.iter_mut();
        let blocks = self.0.iter().skip(block_address as usize);
        for (block, data) in blocks.zip(data.by_ref()) {
            data.copy_from_slice(block);
        }
        if data.next().is_some() {
            Err(MockError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    async fn write(
        &mut self,
        block_address: u32,
        data: &[Aligned<Self::Align, [u8; BLOCK_BYTES]>],
    ) -> Result<(), Self::Error> {
        loog::debug!(
            "writing {} block(s), starting at {block_address}",
            data.len()
        );

        let mut data = data.iter();
        let blocks = self.0.iter_mut().skip(block_address as usize);
        for (block, data) in blocks.zip(data.by_ref()) {
            block.copy_from_slice(data.as_slice());
        }
        if data.next().is_some() {
            Err(MockError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    async fn size(&mut self) -> Result<u64, Self::Error> {
        Ok((BLOCK_BYTES * LEN) as u64)
    }
}
