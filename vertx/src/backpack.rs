use embassy_executor::{task, Spawner};
use embedded_io_async::{Read, Write};
use postcard::accumulator::{CobsAccumulator, FeedResult};

pub(crate) trait Backpack {
    type Rx: Read;
    type Tx: Write;

    fn split(self) -> (Self::Rx, Self::Tx);
}

type BackpackImpl = impl Backpack;

pub(crate) fn run(spawner: Spawner, backpack: BackpackImpl) {
    let backpack = backpack.split();
    spawner.must_spawn(rx(backpack.0));
    spawner.must_spawn(tx(backpack.1));
}

#[task]
async fn rx(rx: BackpackImpl::Rx) {
    let mut raw_buffer = [0; 32];
    let mut accumulator = CobsAccumulator::<256>::new();

    loop {
        let mut chunk = match rx.read(&mut raw_buffer) {
            Ok(len) => &raw_buffer[0..len],
            Err(err) => {
                log::error!("Backpack rx failed: {err:?}");
                continue;
            }
        };

        while !chunk.is_empty() {
            chunk = match accumulator.feed::<()>(&chunk) {
                FeedResult::Consumed => break,
                FeedResult::OverFull(remaining) => remaining,
                FeedResult::DeserError(remaining) => {
                    log::warn!("Backpack rx decode failed");
                    remaining
                }
            }
        }
    }
}

#[task]
async fn tx(tx: BackpackImpl::Tx) {
    todo!()
}
