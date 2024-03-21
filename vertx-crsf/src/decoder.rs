use alloc::ffi::CString;
use core::ops::{Deref, DerefMut};

use crate::{Address, BASIC_CRC};

#[derive(Debug)]
pub enum DecodeError {
    UnexpectedEof,
    InvalidSyncByte(u8),
    InvalidPacketKind(u8),
    BadCrc,
    InvalidExtendedTo,
    InvalidExtendedFrom,
    UnsupportedPacket,
}

#[derive(Debug)]
pub(crate) struct Decoder<'a> {
    payload: &'a [u8],
    /// Next byte index to read from
    next: usize,
    packet_type: u8,
}

impl<'a> Decoder<'a> {
    pub(crate) fn new(buffer: &'a [u8]) -> Result<Self, DecodeError> {
        let [sync, len, packet_type, payload @ ..] = buffer else {
            return Err(DecodeError::UnexpectedEof);
        };
        let (sync, len, packet_type) = (*sync, *len as usize, *packet_type);

        if sync != 0xC8 {
            return Err(DecodeError::InvalidSyncByte(sync));
        }

        // len includes packet_type
        if payload.len() < (len - 1) {
            return Err(DecodeError::UnexpectedEof);
        }

        // len includes packet_type, so subtract 1
        let Some([payload @ .., checksum]) = payload.get(..len - 1) else {
            return Err(DecodeError::UnexpectedEof);
        };

        let mut crc = BASIC_CRC.digest();
        crc.update(&[packet_type]);
        crc.update(payload);

        if crc.finalize() != *checksum {
            return Err(DecodeError::BadCrc);
        }

        Ok(Self {
            payload,
            next: 0,
            packet_type,
        })
    }

    pub(crate) fn extended<'r>(&'r mut self) -> Result<ExtendedDecoder<'r, 'a>, DecodeError> {
        ExtendedDecoder::new(self)
    }

    pub(crate) fn len(&self) -> usize {
        // sync, len, type, payload, checksum
        self.payload_len() + 4
    }

    pub(crate) fn payload_len(&self) -> usize {
        self.payload.len()
    }

    pub(crate) fn packet_type(&self) -> u8 {
        self.packet_type
    }

    pub(crate) fn payload(&mut self) -> &[u8] {
        let rest = &self.payload[self.next..];
        self.next = self.payload.len();
        rest
    }

    pub(crate) fn u8(&mut self) -> u8 {
        let x = u8::from_be(self.payload[self.next]);
        self.next += 1;
        x
    }

    pub(crate) fn i8(&mut self) -> i8 {
        self.u8() as i8
    }

    pub(crate) fn u16(&mut self) -> u16 {
        let bytes = [self.payload[self.next], self.payload[self.next + 1]];
        self.next += 2;
        u16::from_be_bytes(bytes)
    }

    pub(crate) fn i16(&mut self) -> i16 {
        self.u16() as i16
    }

    pub(crate) fn i24(&mut self) -> i32 {
        let b1 = self.payload[self.next];
        let b2 = self.payload[self.next + 1];
        let b3 = self.payload[self.next + 2];
        let b4 = if b3.leading_ones() > 0 { 0xFF } else { 0x00 };
        self.next += 3;

        i32::from_be_bytes([b1, b2, b3, b4])
    }

    pub(crate) fn u32(&mut self) -> u32 {
        let bytes = [
            self.payload[self.next],
            self.payload[self.next + 1],
            self.payload[self.next + 2],
            self.payload[self.next + 3],
        ];
        self.next += 4;
        u32::from_be_bytes(bytes)
    }

    pub(crate) fn i32(&mut self) -> i32 {
        self.u32() as i32
    }

    pub(crate) fn string(&mut self) -> CString {
        let end = self.payload[self.next..]
            .iter()
            .position(|&b| b == 0)
            .map_or(self.payload.len(), |len| self.next + len);
        let string = CString::new(&self.payload[self.next..end]).unwrap();
        self.next = end + 1;
        string
    }
}

impl Drop for Decoder<'_> {
    fn drop(&mut self) {
        #[cfg(test)]
        let is_panicking = std::thread::panicking();
        #[cfg(not(test))]
        let is_panicking = false;

        if cfg!(debug_assertions) && !is_panicking && self.next != self.payload_len() {
            panic!(
                "Expected to read {} bytes. Actually read {} bytes",
                self.payload_len(),
                self.next
            );
        }
    }
}

#[derive(Debug)]
pub(crate) struct ExtendedDecoder<'decoder, 'data> {
    decoder: &'decoder mut Decoder<'data>,
    to: Address,
    from: Address,
}

impl<'decoder, 'data> ExtendedDecoder<'decoder, 'data> {
    fn new(decoder: &'decoder mut Decoder<'data>) -> Result<Self, DecodeError> {
        let to = Address::from_raw(decoder.u8()).ok_or(DecodeError::InvalidExtendedTo)?;
        let from = Address::from_raw(decoder.u8()).ok_or(DecodeError::InvalidExtendedFrom)?;

        Ok(Self { decoder, to, from })
    }

    pub(crate) fn packet_type(&self) -> u8 {
        self.decoder.packet_type()
    }

    pub(crate) fn to(&self) -> Address {
        self.to
    }

    pub(crate) fn from(&self) -> Address {
        self.from
    }
}

impl<'data> Deref for ExtendedDecoder<'_, 'data> {
    type Target = Decoder<'data>;

    fn deref(&self) -> &Self::Target {
        self.decoder
    }
}

impl DerefMut for ExtendedDecoder<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.decoder
    }
}
