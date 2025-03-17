pub(crate) mod api;

use embassy_sync::signal::Signal;

pub(crate) use self::api::Api;

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

// Putting this in the hal implementation would be nicer, but declaring a task
// inside the hal runs into issues with unconstrained use of some of the HAL's
// TAIT definitions that are contained within `Api`/its fields
#[cfg(not(feature = "network"))]
#[embassy_executor::task]
pub(crate) async fn run(api: &'static Api, mut hal: crate::hal::Configurator) -> ! {
    use crate::hal::prelude::*;

    hal.start().await;

    loop {
        let (route, method, writer) = hal.receive().await;
        loog::unwrap!(api.handle(route.as_ref(), method, writer).await);
    }
}
