#![allow(clippy::host_endian_bytes)]
#![warn(clippy::big_endian_bytes, clippy::little_endian_bytes)]

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use embassy_sync::mutex::{Mutex, MutexGuard};
use serde::{Deserialize, Serialize};

use crate::configurator::WifiConfig;
use crate::flash::Partition;

pub struct Manager {
    partition: Partition,
    config: Mutex<crate::mutex::MultiCore, Config>,
    boot_config: Config,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub name: String,
    pub wifi: WifiConfig,
}

impl Manager {
    pub fn new(partitions: &mut Vec<Partition>) -> Self {
        let partition = partitions.iter().position(Partition::is_config).unwrap();
        let partition = partitions.remove(partition);

        let mut length = [0; 1];
        partition.read_into(0, &mut length).unwrap();
        let [length] = length;

        let config = if length == u32::MAX {
            Config::default()
        } else {
            let mut config = vec![0; length as usize / 4];
            partition.read_into(1, &mut config).unwrap();
            let config_bytes = bytemuck::cast_slice(&config);

            serde_json::from_slice(config_bytes).unwrap()
        };

        log::info!("config = {:?}", config);

        Self {
            partition,
            config: Mutex::new(config.clone()),
            boot_config: config,
        }
    }

    #[allow(unused)]
    pub async fn save(&mut self) {
        let encoded = {
            let config = self.config().await;
            serde_json::to_vec(&*config).unwrap()
        };

        let mut data = vec![0; 1 + encoded.len().div_ceil(4)];
        data[0] = encoded.len() as u32;
        bytemuck::cast_slice_mut(&mut data)[4..(4 + encoded.len())].copy_from_slice(&encoded);

        self.partition.erase_and_write(0, &data).unwrap();
    }

    pub async fn config(&mut self) -> MutexGuard<'_, crate::mutex::MultiCore, Config> {
        self.config.lock().await
    }

    pub fn boot_config(&self) -> &Config {
        &self.boot_config
    }
}
