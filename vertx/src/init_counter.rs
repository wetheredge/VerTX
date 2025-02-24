use core::{future, mem, task};

use embassy_sync::waitqueue::AtomicWaker;
use portable_atomic::{AtomicI8, Ordering};

pub(crate) struct InitCounter {
    counter: AtomicI8,
    waker: AtomicWaker,
}

impl InitCounter {
    pub(crate) const fn new() -> Self {
        Self {
            counter: AtomicI8::new(0),
            waker: AtomicWaker::new(),
        }
    }

    pub(crate) fn start(&'static self, task: loog::IStr) -> Tracker {
        self.counter.add(1, Ordering::Relaxed);
        loog::trace!("Starting task: {task=istr}");
        Tracker {
            counter: self,
            task,
        }
    }

    pub(crate) fn wait(&self) -> impl Future<Output = ()> + use<'_> {
        future::poll_fn(|ctx| {
            if self.counter.load(Ordering::Relaxed) == 0 {
                task::Poll::Ready(())
            } else {
                self.waker.register(ctx.waker());
                task::Poll::Pending
            }
        })
    }
}

#[must_use]
pub(crate) struct Tracker {
    counter: &'static InitCounter,
    task: loog::IStr,
}

impl Tracker {
    pub(crate) fn finish(self) {
        let old = self.counter.counter.fetch_sub(1, Ordering::Relaxed);
        loog::trace!("Task initialized: {=istr}", self.task);

        // fetch_sub returns the value before the subtraction
        if old == 1 {
            self.counter.waker.wake();
        }

        mem::forget(self);
    }
}

impl Drop for Tracker {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        loog::unreachable!("Never called .finish() in {} task", (self.task));
    }
}
