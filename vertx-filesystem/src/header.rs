use core::mem;

use aligned::Alignment;
use bytemuck::{Pod, Zeroable};
use crc::Crc;
#[cfg(feature = "defmt")]
use loog::defmt;

use crate::{BLOCK_BYTES, Block, MAX_MODELS, MODEL_BLOCKS, MODELS_START};

static CRC: Crc<u32> = Crc::<u32>::new(&crc::CRC_32_CKSUM);

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub(super) struct Header {
    version: u8,
    _padding0: [u8; 3],
    models: [Model; MAX_MODELS],
    _padding1: [u8; 248],
    checksum: u32,
}

bitfield::bitfield! {
    #[derive(Clone, Copy, PartialEq, Pod, Zeroable)]
    #[repr(transparent)]
    pub(crate) struct Model(u32);
    impl Debug;
    pub(crate) start, set_start: 19, 0;
    pub(crate) u8, len, set_len: 23, 20;
    pub(crate) u8, id, set_id: 29, 24;
    // unused: 31, 30;
}

const _: () = assert!(mem::size_of::<Header>() == BLOCK_BYTES);

#[derive(Debug, Clone, PartialEq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    Missing,
    Version,
    Checksum,
}

impl Header {
    pub(crate) fn from_block<A: Alignment>(block: &Block<A>) -> &Self {
        bytemuck::must_cast_ref(block.as_words())
    }

    pub(crate) fn from_block_mut<A: Alignment>(block: &mut Block<A>) -> &mut Self {
        bytemuck::must_cast_mut(block.as_words_mut())
    }

    pub(crate) fn checksum(&self) -> u32 {
        let mut crc = CRC.digest();
        crc.update(&[self.version]);
        crc.update(bytemuck::must_cast_slice(&self.models));
        crc.finalize()
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.version == 0 {
            Err(Error::Missing)
        } else if self.version != 1 {
            Err(Error::Version)
        } else if u32::from_le(self.checksum) != self.checksum() {
            Err(Error::Checksum)
        } else {
            Ok(())
        }
    }

    pub(crate) fn init(&mut self) {
        self.version = 1;
        // NB: checksum will be updated before writing
    }

    pub(crate) fn iter_models(&self) -> impl Iterator<Item = &Model> {
        self.models.iter().take_while(|model| !model.is_empty())
    }

    #[expect(clippy::manual_inspect)]
    pub(crate) fn new_model(&mut self) -> Option<&mut Model> {
        let mut used: IdSet = 0;
        for model in self.iter_models() {
            used &= 1 << IdSet::from(model.id());
        }

        loog::trace!("used model ids: {used=u64:b}");

        let id = next_id(used)?;
        self.models.iter_mut().find(|m| m.is_empty()).map(|model| {
            model.set_start(u32::from(id) * MODEL_BLOCKS + MODELS_START);
            model.set_len(MODEL_BLOCKS as u8);
            model.set_id(id);
            model
        })
    }

    pub(crate) fn delete_model(&mut self, id: u8) {
        let Some(index) = self.iter_models().position(|model| model.id() == id) else {
            loog::warn!("there is no model with id {id=u8}");
            return;
        };

        let end = self
            .iter_models()
            .skip(index)
            .position(Model::is_empty)
            .map(|i| i + index)
            .unwrap_or(self.models.len());

        self.models.copy_within((index + 1)..end, index);
        self.models[end - 1] = Model::EMPTY;
    }
}

impl Model {
    const EMPTY: Self = Self(0);

    fn is_empty(&self) -> bool {
        self.start() == 0
    }

    pub(crate) fn end(&self) -> u32 {
        self.start() + u32::from(self.len())
    }
}

type IdSet = u64;
const _: () = assert!((mem::size_of::<IdSet>() * 8) == MAX_MODELS);

fn next_id(used: IdSet) -> Option<u8> {
    let zeros = used.leading_zeros();
    if zeros == 0 {
        let id = used.trailing_ones();
        ((id as usize) < MAX_MODELS).then_some(id as u8)
    } else {
        Some((MAX_MODELS as u32 - zeros) as u8)
    }
}

#[cfg(test)]
mod tests {
    use aligned::A1;

    use super::*;

    fn le(x: u32) -> u32 {
        x.to_le()
    }

    fn model(start: u32, len: u8, id: u8) -> Model {
        let mut model = Model::EMPTY;
        model.set_start(start);
        model.set_len(len);
        model.set_id(id);
        model
    }

    #[test_log::test]
    fn missing() {
        let block = Block::<A1>::new();
        let header = Header::from_block(&block);
        assert_eq!(header.validate(), Err(Error::Missing));
    }

    #[test_log::test]
    fn version() {
        let mut block = Block::<A1>::new();
        block.as_words_mut()[0] = le(2);

        let header = Header::from_block(&block);
        assert_eq!(header.validate(), Err(Error::Version));
    }

    #[test_log::test]
    fn checksum() {
        let mut block = Block::<A1>::new();
        {
            let block = block.as_words_mut();
            block[0] = le(1);
            block[127] = le(0xDEAD_BEEF);
        }

        let header = Header::from_block(&block);
        assert_eq!(header.validate(), Err(Error::Checksum));
    }

    #[test_log::test]
    fn valid() {
        let mut block = Block::<A1>::new();
        {
            let block = block.as_words_mut();
            block[0] = le(1);
            block[1] = le(0x2A20_000A);
            block[2] = le(0x3F10_0014);
            block[3] = le(0x0110_0000);
            block[127] = le(0x8EB3_FB6B);
        }

        let header = Header::from_block(&block);
        header.validate().unwrap();

        let mut models = header.iter_models();
        assert_eq!(model(10, 2, 42), *models.next().unwrap());
        assert_eq!(model(20, 1, 63), *models.next().unwrap());
        assert!(models.next().is_none());
    }

    #[test]
    fn model_ids_full() {
        assert_eq!(next_id(u64::MAX), None);
    }

    #[test]
    fn model_ids_next() {
        assert_eq!(next_id(0b0100_0000), Some(7));
    }

    #[test]
    fn model_ids_wrapping() {
        assert_eq!(next_id(0xFFFF_FFFF_FFFF_FFFB), Some(2));
    }
}
