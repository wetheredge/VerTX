use embassy_sync::pubsub::{self, PubSubChannel};

const SUBS: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Ok,
    Armed,
    PreWiFi,
    WiFi,
    Updating,
}

pub struct Channel(PubSubChannel<crate::mutex::MultiCore, Mode, 1, SUBS, 0>);

impl Channel {
    pub const fn new() -> Self {
        Self(PubSubChannel::new())
    }

    pub fn publisher(&self) -> Publisher<'_> {
        Publisher(self.0.immediate_publisher())
    }

    pub fn subscriber(&self) -> Option<Subscriber<'_>> {
        self.0.subscriber().ok().map(Subscriber)
    }
}

pub struct Publisher<'a>(pubsub::ImmediatePublisher<'a, crate::mutex::MultiCore, Mode, 1, SUBS, 0>);

impl Publisher<'_> {
    pub fn publish(&self, mode: Mode) {
        self.0.publish_immediate(mode);
    }
}

pub struct Subscriber<'a>(pubsub::Subscriber<'a, crate::mutex::MultiCore, Mode, 1, SUBS, 0>);

impl Subscriber<'_> {
    pub async fn next(&mut self) -> Mode {
        self.0.next_message_pure().await
    }
}
