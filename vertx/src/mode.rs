use embassy_sync::pubsub::{self, PubSubChannel};

const SUBS: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Ok,
    #[allow(unused)]
    Armed,
    PreConfigurator,
    Configurator,
    #[allow(unused)]
    Updating,
}

pub struct Channel(PubSubChannel<crate::mutex::MultiCore, Mode, 1, SUBS, 0>);

impl Channel {
    pub const fn new() -> Self {
        Self(PubSubChannel::new())
    }

    pub fn subscriber(&self) -> Option<Subscriber<'_>> {
        self.0.subscriber().ok().map(Subscriber)
    }

    pub fn publish(&self, mode: Mode) {
        self.0.immediate_publisher().publish_immediate(mode);
    }
}

pub struct Subscriber<'a>(pubsub::Subscriber<'a, crate::mutex::MultiCore, Mode, 1, SUBS, 0>);

impl Subscriber<'_> {
    pub async fn next(&mut self) -> Mode {
        self.0.next_message_pure().await
    }
}
