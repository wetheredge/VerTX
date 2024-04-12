use alloc::vec::Vec;

use crc::Crc;
use embassy_sync::signal::Signal;
use picoserve::io::Read;
use picoserve::request::RequestBodyReader;
use vertx_api::response;

use crate::flash::{
    AppPartitionKind, DataPartitionKind, Partition, PartitionKind, SECTOR_BYTES, SECTOR_WORDS,
};

pub struct Manager {
    updated: bool,
    ota_data: Partition,
    has_factory_app: bool,
    ota_partitions: Vec<Option<Partition>>,
}

impl Manager {
    pub fn new(partitions: Vec<Partition>) -> Self {
        let mut ota_data = None;
        let mut has_factory_app = false;
        let mut ota_partitions = Vec::new();

        for partition in partitions {
            match partition.kind {
                PartitionKind::App(AppPartitionKind::Factory) => has_factory_app = true,
                PartitionKind::App(kind) if kind.is_ota() => ota_partitions.push(Some(partition)),
                PartitionKind::Data(DataPartitionKind::Ota) => ota_data = Some(partition),
                _ => {}
            }
        }

        assert!(ota_partitions.len() >= 2);

        Self {
            updated: false,
            ota_data: ota_data.unwrap(),
            has_factory_app,
            ota_partitions,
        }
    }

    pub async fn update<R: Read>(
        &mut self,
        mut reader: RequestBodyReader<'_, R>,
        progress: &Signal<crate::mutex::SingleCore, response::UpdateProgress>,
    ) {
        assert!(!self.updated);
        self.updated = true;

        let mut ota_state = OtaState::new(
            &mut self.ota_data,
            self.has_factory_app,
            &mut self.ota_partitions,
        )
        .unwrap();

        let mut buffer = [0; SECTOR_WORDS as usize];
        let mut offset_words = 0;
        let mut done = false;
        while !done {
            let buffer_bytes: &mut [u8; SECTOR_BYTES as usize] = bytemuck::cast_mut(&mut buffer);

            // Equivalent to read_exact, but zeros the leftover space at the end
            let mut i = 0;
            while i < buffer_bytes.len() {
                let read = reader.read(&mut buffer_bytes[i..]).await.unwrap();
                if read == 0 {
                    buffer_bytes[i..].fill(0);
                    done = true;
                    break;
                }
                i += read;
            }

            ota_state
                .next_partition
                .write(offset_words, &buffer)
                .unwrap();
            offset_words += SECTOR_WORDS;
            progress.signal(response::UpdateProgress {
                written: offset_words * 4,
            });
        }

        ota_state.update_otadata().unwrap();
    }
}

#[derive(Debug)]
struct OtaState<'a> {
    ota_data: &'a mut Partition,
    next_partition: Partition,
    next_data: OtaData,
    next_sequence: u32,
}

impl<'a> OtaState<'a> {
    fn new(
        ota_data: &'a mut Partition,
        has_factory_app: bool,
        ota_partitions: &'a mut [Option<Partition>],
    ) -> Result<Self, i32> {
        let ota_data0 = OtaData::new(ota_data, Channel::First)?;
        let ota_data1 = OtaData::new(ota_data, Channel::Second)?;

        let second_is_newer = ota_data1.sequence > ota_data0.sequence;
        let only_factory_installed = has_factory_app && ota_data0.sequence == ota_data1.sequence;
        let (next_data, current_sequence) = if only_factory_installed || second_is_newer {
            (ota_data0, ota_data1.sequence)
        } else {
            (ota_data1, ota_data0.sequence)
        };

        let next_partition_id = current_sequence as usize % ota_partitions.len();
        Ok(Self {
            ota_data,
            next_partition: ota_partitions[next_partition_id].take().unwrap(),
            next_data,
            next_sequence: current_sequence + 1,
        })
    }

    fn update_otadata(&mut self) -> Result<(), i32> {
        self.next_data
            .update_otadata(self.ota_data, self.next_sequence)
    }
}

// https://github.com/bjoernQ/esp32c3-ota-experiment/blob/ad812ee648e8efa9469c2b48044ce7de18975167/src/ota.rs#L5-L14
const CRC: Crc<u32> = Crc::<u32>::new(&crc::Algorithm {
    width: 32,
    poly: 0x04C11DB7,
    init: 0,
    refin: true,
    refout: true,
    xorout: u32::MAX,
    check: 0,
    residue: 0,
});

#[derive(Debug)]
struct OtaData {
    offset: u32,
    sequence: u32,
    buffer: [u32; 8],
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
enum Channel {
    First,
    Second,
}

impl Channel {
    const fn ota_offset(self) -> u32 {
        self as u32 * SECTOR_WORDS
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
enum State {
    New,
    PendingVerify,
    Valid,
    Invalid,
    Aborted,
    Undefined = u32::MAX,
}

impl From<State> for u32 {
    fn from(state: State) -> Self {
        state as u32
    }
}

#[allow(clippy::host_endian_bytes)]
#[warn(clippy::big_endian_bytes, clippy::little_endian_bytes)]
impl OtaData {
    fn new(ota_data: &Partition, channel: Channel) -> Result<Self, i32> {
        let mut buffer = [0; 8];
        let offset = channel.ota_offset();
        ota_data.read_into(offset, &mut buffer).unwrap();
        esp_println::dbg!(&buffer);

        let sequence = buffer[0];

        Ok(Self {
            offset,
            sequence: if sequence == u32::MAX { 0 } else { sequence },
            buffer,
        })
    }

    fn update_otadata(&mut self, ota_data: &mut Partition, sequence: u32) -> Result<(), i32> {
        self.buffer[0] = sequence;
        self.buffer[6] = u32::from(State::Valid);
        self.buffer[7] = CRC.checksum(&sequence.to_ne_bytes());

        esp_println::dbg!(&self.buffer);
        ota_data.erase_and_write(self.offset, &self.buffer)
    }
}
