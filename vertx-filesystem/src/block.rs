#![expect(trivial_casts)]

use core::{mem, slice};

use aligned::{Aligned, Alignment};

use crate::BLOCK_BYTES;

#[repr(C, align(4))]
pub(crate) struct Block<A>(Aligned<A, [u8; BLOCK_BYTES]>);

const fn check_layout<A: Alignment>() -> bool {
    mem::size_of::<Block<A>>() == BLOCK_BYTES
        && mem::align_of::<Block<A>>() >= 4
        && mem::align_of::<Block<A>>() >= mem::align_of::<A>()
}

const _: () = assert!(check_layout::<aligned::A1>());
const _: () = assert!(check_layout::<aligned::A4>());
const _: () = assert!(check_layout::<aligned::A16>());

impl<A: Alignment> Block<A> {
    pub(crate) const fn new() -> Self {
        Self(Aligned([0; BLOCK_BYTES]))
    }
}

// SAFETY: these rely on these types (and similarly their slices) having the
// same layout:
// - [u8; X]
// - [u32; X / 4]
// - Aligned<_, [u8; X]>
#[expect(clippy::undocumented_unsafe_blocks)]
impl<A: Alignment> Block<A> {
    pub(crate) const fn as_words(&self) -> &[u32; BLOCK_BYTES / 4] {
        unsafe { &*(&self.0 as *const _ as *const _) }
    }

    pub(crate) const fn as_words_mut(&mut self) -> &mut [u32; BLOCK_BYTES / 4] {
        unsafe { &mut *(&mut self.0 as *mut _ as *mut _) }
    }

    pub(crate) const fn as_aligned(&self) -> &[Aligned<A, [u8; BLOCK_BYTES]>] {
        let aligned: &Aligned<A, [u8; BLOCK_BYTES]> =
            unsafe { &*(&self.0 as *const _ as *const _) };
        slice::from_ref(aligned)
    }

    pub(crate) const fn as_aligned_mut(&mut self) -> &mut [Aligned<A, [u8; BLOCK_BYTES]>] {
        let aligned: &mut Aligned<A, [u8; BLOCK_BYTES]> =
            unsafe { &mut *(&mut self.0 as *mut _ as *mut _) };
        slice::from_mut(aligned)
    }

    pub(crate) const fn as_byte_slice(this: &[Self]) -> &[u8] {
        unsafe { slice::from_raw_parts(this.as_ptr().cast(), this.len() * BLOCK_BYTES) }
    }

    pub(crate) const fn as_byte_slice_mut(this: &mut [Self]) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(this.as_mut_ptr().cast(), this.len() * BLOCK_BYTES) }
    }

    pub(crate) const fn as_word_slice(this: &[Self]) -> &[u32] {
        unsafe { slice::from_raw_parts(this.as_ptr().cast(), this.len() * (BLOCK_BYTES / 4)) }
    }

    pub(crate) const fn as_word_slice_mut(this: &mut [Self]) -> &mut [u32] {
        unsafe {
            slice::from_raw_parts_mut(this.as_mut_ptr().cast(), this.len() * (BLOCK_BYTES / 4))
        }
    }

    pub(crate) const fn as_aligned_slice_mut(
        this: &mut [Self],
    ) -> &mut [Aligned<A, [u8; BLOCK_BYTES]>] {
        unsafe { &mut *(this as *mut _ as *mut _) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! each_alignment {
        ([$($a:ident)+] |$var:ident, $A:ident| $test:tt) => {$({
            type $A = aligned::$a;
            $test;
        })+};
        ($($t:tt)+) => {
            each_alignment!([A1 A2 A4 A8 A16 A32] $($t)+);
        };
    }

    #[test]
    fn words() {
        each_alignment!(|block, A| {
            let mut block = Block::<A>::new();
            block.as_words_mut()[1] = u32::to_le(0xDEADBEEF);
            assert_eq!(u32::from_le(block.as_words()[1]), 0xDEADBEEF);
        });
    }

    #[test]
    fn aligned() {
        each_alignment!(|block, A| {
            let mut block = Block::<A>::new();
            block.as_aligned_mut()[0][7] = 42;
            assert_eq!(block.as_aligned()[0][7], 42);
        });
    }

    #[test]
    fn byte_slice() {
        each_alignment!(|block, A| {
            let blocks = &mut [const { Block::<A>::new() }; 2];

            let bytes = Block::as_byte_slice_mut(blocks);
            assert_eq!(bytes.len(), BLOCK_BYTES * 2);
            bytes[7] = 1;
            bytes[BLOCK_BYTES] = 2;
            bytes[BLOCK_BYTES * 2 - 1] = 3;
            assert!(bytes.get(BLOCK_BYTES * 2).is_none());

            let bytes = Block::as_byte_slice(blocks);
            assert_eq!(bytes.len(), BLOCK_BYTES * 2);
            assert_eq!(bytes[7], 1);
            assert_eq!(bytes[BLOCK_BYTES], 2);
            assert_eq!(bytes[BLOCK_BYTES * 2 - 1], 3);
            assert!(bytes.get(BLOCK_BYTES * 2).is_none());
        });
    }

    #[test]
    fn word_slice() {
        each_alignment!(|block, A| {
            let blocks = &mut [const { Block::<A>::new() }; 2];

            let words = Block::as_word_slice_mut(blocks);
            assert_eq!(words.len(), BLOCK_BYTES / 2);
            words[7] = 0x1200_0034;
            words[words.len() - 1] = 0x5678_9ABC;
            assert!(words.get(BLOCK_BYTES / 2).is_none());

            let words = Block::as_word_slice(blocks);
            assert_eq!(words.len(), BLOCK_BYTES / 2);
            assert_eq!(words[7], 0x1200_0034);
            assert_eq!(words[words.len() - 1], 0x5678_9ABC);
            assert!(words.get(BLOCK_BYTES / 2).is_none());
        });
    }

    #[test]
    fn aligned_slice() {
        each_alignment!(|block, A| {
            let blocks = &mut [const { Block::<A>::new() }; 2];
            Block::as_aligned_slice_mut(blocks)[1][7] = 42;
            assert_eq!(Block::as_byte_slice(blocks)[7], 0);
            assert_eq!(Block::as_byte_slice(blocks)[BLOCK_BYTES + 7], 42);
        });
    }
}
