#![allow(clippy::host_endian_bytes)]
#![warn(clippy::big_endian_bytes, clippy::little_endian_bytes)]

use core::ops::Deref;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::flash::Partition;
use crate::wifi::Config as WifiConfig;

#[derive(Debug)]
pub struct ConfigManager {
    partition: Partition,
    config: Config,
}

impl Deref for ConfigManager {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub name: String,
    pub wifi: WifiConfig,
}

impl Config {
    pub fn load(partitions: &mut Vec<Partition>) -> ConfigManager {
        let partition = partitions.iter().position(Partition::is_config).unwrap();
        let partition = partitions.swap_remove(partition);

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

        ConfigManager { partition, config }
    }
}

impl ConfigManager {
    #[allow(unused)]
    fn save(&mut self) {
        let encoded = serde_json::to_vec(&self.config).unwrap();

        let mut data = vec![0; 1 + encoded.len().div_ceil(4)];
        data[0] = encoded.len() as u32;
        bytemuck::cast_slice_mut(&mut data)[4..(4 + encoded.len())].copy_from_slice(&encoded);

        self.partition.erase_and_write(0, &data).unwrap();
    }
}
