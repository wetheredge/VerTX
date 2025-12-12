use aligned::Alignment;
use block_device_driver::BlockDevice;

use crate::{BLOCK_BYTES, Block};

pub(crate) struct Buffer<'a, A, const LEN: usize> {
    buffers: &'a mut [Block<A>; LEN],
    state: [State; LEN],
}

struct State(u32);

impl State {
    const BLOCK_MASK: u32 = !Self::MODIFIED_MASK;
    const EMPTY: Self = Self(u32::MAX);
    const MODIFIED_BIT: u32 = 31;
    const MODIFIED_MASK: u32 = 1 << Self::MODIFIED_BIT;

    const fn is_empty(&self) -> bool {
        self.0 == u32::MAX
    }

    fn set_empty(&mut self) {
        self.0 = u32::MAX;
    }

    fn block(&self) -> Option<u32> {
        (self.0 != u32::MAX).then_some(self.0 & Self::BLOCK_MASK)
    }

    fn set_block(&mut self, block: u32) {
        self.0 = block;
        debug_assert!(!self.is_modified());
    }

    fn is_block(&self, block: u32) -> bool {
        debug_assert!(block & Self::MODIFIED_MASK == 0);
        (self.0 & Self::MODIFIED_MASK) == block
    }

    fn is_modified(&self) -> bool {
        (!self.is_empty()) && ((self.0 & Self::MODIFIED_MASK) != 0)
    }

    fn clear_modified(&mut self) {
        if !self.is_empty() {
            self.0 &= Self::BLOCK_MASK;
        }
    }

    fn set_modified(&mut self) {
        self.0 |= Self::MODIFIED_MASK;
    }
}

impl<'a, A, const LEN: usize> Buffer<'a, A, LEN> {
    pub(crate) const fn new(buffer: &'a mut [Block<A>; LEN]) -> Self {
        Self {
            buffers: buffer,
            state: [State::EMPTY; LEN],
        }
    }

    /// Length in blocks
    pub(crate) const fn len(&self) -> u32 {
        LEN as u32
    }
}

impl<A: Alignment, const LEN: usize> Buffer<'_, A, LEN> {
    /// Select at least 1 buffer starting at block `start`. Will only return
    /// multiple buffers if they already contain the correct blocks.
    pub(crate) async fn select<'a, D: BlockDevice<BLOCK_BYTES, Align = A>>(
        &'a mut self,
        device: &'a mut D,
        start: u32,
    ) -> Result<View<'a, A, D>, D::Error> {
        loog::trace!("requesting buffer view at block {start=u32}");

        let (index, len) =
            if let Some(start_index) = self.state.iter().position(|state| state.is_block(start)) {
                let mut len: usize = 1;
                while let Some(state) = self.state.get(start_index + len)
                    && state.is_block(start + len as u32)
                {
                    len += 1;
                }

                (start_index, len)
            } else if let Some(prev) = start.checked_sub(1)
                && let Some(prev_index) = self
                    .state
                    .iter()
                    .take(LEN - 1)
                    .position(|state| state.is_block(prev))
            {
                // Try to extend existing data to get efficient rewinds
                (prev_index + 1, 1)
            } else {
                (0, 1)
            };

        View::new(device, self, start, index, index + len).await
    }

    /// Select exactly `len` blocks worth of buffers.
    pub(crate) async fn select_exact<'a, D: BlockDevice<BLOCK_BYTES, Align = A>>(
        &'a mut self,
        device: &'a mut D,
        start: u32,
        len: u32,
    ) -> Result<View<'a, A, D>, D::Error> {
        loog::trace!("requesting view of exactly {len=u32} buffers at block {start=u32}");

        debug_assert!(LEN >= len as usize);

        let index = self
            .state
            .iter()
            .take(LEN - len as usize + 1)
            .position(|state| state.is_block(start))
            .unwrap_or(0);

        View::new(device, self, start, index, index + len as usize).await
    }

    pub(crate) async fn flush<D: BlockDevice<BLOCK_BYTES, Align = A>>(
        &mut self,
        dev: &mut D,
    ) -> Result<(), D::Error> {
        let mut start = 0;
        loop {
            // Skip past any unmodified buffers
            while self
                .state
                .get(start)
                .is_some_and(|state| !state.is_modified())
            {
                start += 1;
            }

            if start >= LEN {
                return Ok(());
            }

            let start_block = self.state[start]
                .block()
                .expect("block() is always Some(_) when is_modified() == true");

            // Find the chunk of modified buffers backed by a contiguous range of blocks
            let mut chunk: usize = 1;
            while let state = &self.state[start + chunk]
                && state.is_modified()
                && state.is_block(start_block + chunk as u32)
            {
                chunk += 1;
            }

            let end = start + chunk;

            let data = &mut self.buffers[start..end];
            dev.write(start_block, Block::as_aligned_slice_mut(data))
                .await?;

            // Do this after writing in case the future gets cancelled
            for state in &mut self.state[start..end] {
                state.clear_modified();
            }

            start += chunk;
        }
    }
}

pub(crate) struct View<'a, A, D> {
    device: &'a mut D,
    buffers: &'a mut [Block<A>],
    state: &'a mut [State],
    start: u32,
}

impl<A, D> View<'_, A, D> {
    pub(crate) fn data(&self) -> &[Block<A>] {
        self.buffers
    }

    pub(crate) fn data_mut(&mut self) -> &mut [Block<A>] {
        self.buffers
    }
}

impl<'a, A: Alignment, D: BlockDevice<BLOCK_BYTES, Align = A>> View<'a, A, D> {
    async fn new<const LEN: usize>(
        device: &'a mut D,
        buffer: &'a mut Buffer<'_, A, LEN>,
        start: u32,
        start_index: usize,
        end_index: usize,
    ) -> Result<Self, D::Error> {
        loog::trace!("viewing buffers {start_index}..{end_index} at block {start=u32}");

        let buffers = &mut buffer.buffers[start_index..end_index];
        let state = &mut buffer.state[start_index..end_index];

        // Point all buffers to the right blocks, flushing modified data if needed
        for i in 0..state.len() {
            let state = &mut state[i];
            let expected_block = start + i as u32;

            if let Some(block) = state.block()
                && block != expected_block
            {
                // TODO: chunk writes
                device.write(block, buffers[i].as_aligned()).await?;
                state.set_empty();
            }
        }

        Ok(Self {
            device,
            buffers,
            state,
            start,
        })
    }
}

impl<A: Alignment, D: BlockDevice<BLOCK_BYTES, Align = A>> View<'_, A, D> {
    /// Fill the buffers from `dev`
    pub(crate) async fn read(&mut self) -> Result<(), D::Error> {
        // TODO: chunk reads

        for i in 0..self.state.len() {
            let state = &mut self.state[i];
            let buffer = self.buffers[i].as_aligned_mut();
            let block = self.start + i as u32;

            if let Some(current) = state.block() {
                if current == block {
                    continue;
                } else if state.is_modified() {
                    loog::trace!("flushing block {current=u32} from buffer {i}");
                    self.device.write(current, buffer).await?;
                }
            }

            loog::trace!("reading block {block=u32} into buffer {i}");
            self.device.read(block, buffer).await?;
            state.set_block(block);
        }

        Ok(())
    }

    pub(crate) fn mark_modified(&mut self, offset: usize, len: usize) {
        let start = offset / BLOCK_BYTES;
        let end = (offset + len).div_ceil(BLOCK_BYTES);
        for (state, block) in self
            .state
            .iter_mut()
            .zip(self.start..)
            .skip(start)
            .take(end - start)
        {
            state.set_block(block);
            state.set_modified();
        }
    }
}
