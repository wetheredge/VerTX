use core::mem::MaybeUninit;

use super::State;

pub(super) struct History<const N: usize> {
    stack: [MaybeUninit<State>; N],
    depth: usize,
}

impl<const N: usize> History<N> {
    pub(super) const fn new(root: State) -> Self {
        let mut stack = [const { MaybeUninit::uninit() }; N];
        stack[0] = MaybeUninit::new(root);
        Self { stack, depth: 0 }
    }

    pub(super) fn current(&mut self) -> &mut State {
        // SAFETY: `stack` is only accessed at `depth`, which is only incremented after
        // initializing the next entry. `stack[0]` gets initialized in `new()`.
        unsafe { self.stack[self.depth].assume_init_mut() }
    }

    pub(super) const fn is_root(&self) -> bool {
        self.depth == 0
    }

    pub(super) fn push(&mut self, next: State) {
        self.stack[self.depth + 1] = MaybeUninit::new(next);
        self.depth += 1;
    }

    pub(super) fn pop(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }
}
