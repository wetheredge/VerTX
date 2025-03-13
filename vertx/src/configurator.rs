use embassy_sync::signal::Signal;

static START: Signal<crate::mutex::MultiCore, ()> = Signal::new();

#[derive(Debug, Clone, Copy)]
pub(crate) struct Manager {
    _private: (),
}

impl Manager {
    pub(crate) const fn new() -> Self {
        Self { _private: () }
    }

    pub(crate) fn start(&self) {
        START.signal(());
    }

    pub(crate) fn wait(&self) -> impl Future<Output = ()> {
        START.wait()
    }
}
