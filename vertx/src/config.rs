#![allow(clippy::host_endian_bytes)]
#![warn(clippy::big_endian_bytes, clippy::little_endian_bytes)]

use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use embassy_sync::mutex::Mutex;
use vertx_config::update;

use crate::flash::Partition;

pub struct Manager {
    modified: AtomicBool,
    partition: Mutex<crate::mutex::SingleCore, Partition>,
    config: crate::Config,
}

impl Manager {
    pub fn new(partitions: &mut Vec<Partition>) -> Self {
        let partition = partitions.iter().position(Partition::is_config).unwrap();
        let partition = partitions.swap_remove(partition);

        let mut length = [0; 1];
        partition.read_into(0, &mut length).unwrap();
        let [length] = length;
        let length = if length == u32::MAX { 0 } else { length };
        let length = length as usize;

        let config = if length > 0 {
            let mut config = vec![0; length.div_ceil(4)];
            partition.read_into(1, &mut config).unwrap();
            let config_bytes: &[u8] = &bytemuck::cast_slice(&config)[..length];

            match vertx_config::storage::postcard::from_slice(config_bytes) {
                Ok(config) => config,
                Err(err) => {
                    log::error!("Failed to load config: {err}");
                    Default::default()
                }
            }
        } else {
            Default::default()
        };

        Self {
            modified: AtomicBool::new(false),
            partition: Mutex::new(partition),
            config,
        }
    }

    pub async fn save(&self) {
        if !self.modified.swap(false, Ordering::AcqRel) {
            return;
        }

        let encoded = vertx_config::storage::postcard::to_vec(&self.config).await;

        let mut data = vec![0; 1 + encoded.len().div_ceil(4)];
        data[0] = encoded.len() as u32;
        bytemuck::cast_slice_mut(&mut data)[4..(4 + encoded.len())].copy_from_slice(&encoded);

        let mut partition = self.partition.lock().await;
        partition.erase_and_write(0, &data).unwrap();
    }

    pub fn config(&self) -> &crate::Config {
        &self.config
    }
}

impl vertx_config::UpdateRef for Manager {
    async fn update_ref<'a>(&self, key: &'a str, update: update::Update<'a>) -> update::Result {
        // TODO: prevent saving while updating
        self.modified.store(true, Ordering::Release);
        self.config().update_ref(key, update).await?;
        Ok(())
    }
}
