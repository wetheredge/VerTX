use core::mem::MaybeUninit;

use super::State;

pub(super) struct History<const N: usize> {
    stack: [MaybeUninit<State>; N],
    depth: usize,
}

impl<const N: usize> History<N> {
    pub(super) const fn new(root: State) -> Self {
        #[allow(clippy::declare_interior_mutable_const)]
        const INIT: MaybeUninit<State> = MaybeUninit::uninit();
        let mut stack = [INIT; N];
        stack[0] = MaybeUninit::new(root);
        Self { stack, depth: 0 }
    }

    pub(super) fn current(&mut self) -> &mut State {
        let current = &mut self.stack[self.depth];
        unsafe { current.assume_init_mut() }
    }

    pub(super) const fn is_root(&self) -> bool {
        self.depth == 0
    }

    pub(super) fn push(&mut self, next: State) {
        self.depth += 1;
        self.stack[self.depth] = MaybeUninit::new(next);
    }

    pub(super) fn pop(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }
}
