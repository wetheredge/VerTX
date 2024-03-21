use alloc::ffi::CString;

use crate::{Address, BASIC_CRC};

#[derive(Debug)]
pub enum EncodeError {
    BufferTooSmall,
    UnsupportedPacket,
}

#[derive(Debug)]
pub(crate) struct Encoder<'a> {
    buffer: &'a mut [u8],
    /// Next byte index to write to
    next: usize,
    extended: bool,
}

impl<'a> Encoder<'a> {
    pub(crate) fn new(buffer: &'a mut [u8]) -> Result<Self, EncodeError> {
        // sync, len, packet_type, empty payload, crc
        if buffer.len() < 4 {
            return Err(EncodeError::BufferTooSmall);
        }

        buffer[0] = 0xC8;

        Ok(Self {
            buffer,
            next: 3,
            extended: false,
        })
    }

    pub(crate) fn extended(&mut self, to: Address, from: Address) -> Result<(), EncodeError> {
        if self.buffer.len() < (self.next + 2) {
            return Err(EncodeError::BufferTooSmall);
        }

        self.buffer[self.next] = to.into_raw();
        self.buffer[self.next + 1] = from.into_raw();

        self.extended = true;
        self.next += 2;
        Ok(())
    }

    pub(crate) fn finish(self, packet_type: u8) -> Result<usize, EncodeError> {
        // Minus leading sync & len; plus trailing checksum
        self.buffer[1] = self.next as u8 - 2 + 1;
        self.buffer[2] = packet_type;

        let mut crc = BASIC_CRC.digest();
        crc.update(&self.buffer[2..self.next]);

        let Some(target) = self.buffer.get_mut(self.next) else {
            return Err(EncodeError::BufferTooSmall);
        };
        *target = crc.finalize();
        Ok(self.next)
    }

    pub(crate) fn u8(&mut self, value: u8) -> Result<(), EncodeError> {
        let Some(target) = self.buffer.get_mut(self.next) else {
            return Err(EncodeError::BufferTooSmall);
        };
        *target = value;
        self.next += 1;
        Ok(())
    }

    pub(crate) fn i8(&mut self, value: i8) -> Result<(), EncodeError> {
        self.u8(value as u8)
    }

    pub(crate) fn u16(&mut self, value: u16) -> Result<(), EncodeError> {
        let Some(target) = self.buffer.get_mut(self.next..self.next + 2) else {
            return Err(EncodeError::BufferTooSmall);
        };

        let value = value.to_be_bytes();
        target.copy_from_slice(&value);
        self.next += 2;
        Ok(())
    }

    pub(crate) fn i16(&mut self, value: i16) -> Result<(), EncodeError> {
        self.u16(value as u16)
    }

    pub(crate) fn i24(&mut self, value: i32) -> Result<(), EncodeError> {
        let Some(target) = self.buffer.get_mut(self.next..self.next + 3) else {
            return Err(EncodeError::BufferTooSmall);
        };

        let value = value.to_be_bytes();
        target.copy_from_slice(&value[..3]);
        self.next += 3;
        Ok(())
    }

    pub(crate) fn u32(&mut self, value: u32) -> Result<(), EncodeError> {
        let Some(target) = self.buffer.get_mut(self.next..self.next + 4) else {
            return Err(EncodeError::BufferTooSmall);
        };

        let value = value.to_be_bytes();
        target.copy_from_slice(&value);
        self.next += 4;
        Ok(())
    }

    fn i32(&mut self, value: i32) -> Result<(), EncodeError> {
        self.u32(value as u32)
    }

    pub(crate) fn slice(&mut self, slice: &[u8]) -> Result<(), EncodeError> {
        let Some(target) = self.buffer.get_mut(self.next..self.next + slice.len()) else {
            return Err(EncodeError::BufferTooSmall);
        };

        target.copy_from_slice(slice);
        self.next += slice.len();
        Ok(())
    }

    pub(crate) fn string(&mut self, string: &CString) -> Result<(), EncodeError> {
        self.slice(string.to_bytes_with_nul())
    }
}
