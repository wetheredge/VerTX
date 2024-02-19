#![allow(clippy::host_endian_bytes)]
#![warn(clippy::big_endian_bytes, clippy::little_endian_bytes)]

use core::iter;

use alloc::string::String;
use alloc::vec;

use esp_storage::FlashStorage;
use serde::{Deserialize, Serialize};

use crate::wifi::Config as WifiConfig;

const FLASH_START: u32 = 0x9000;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub name: String,
    pub wifi: WifiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: String::from("VHS"),
            wifi: WifiConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        const LENGTH_BYTES: u32 = usize::BITS / 8;
        let mut length = 0;
        unsafe { esp_storage::ll::spiflash_read(FLASH_START, &mut length, LENGTH_BYTES) }.unwrap();

        let config = if length == u32::MAX {
            log::info!("Initializing config");

            let config = Config::default();
            config.save();
            config
        } else {
            let mut config = vec![0_u8; length as usize];
            unsafe {
                esp_storage::ll::spiflash_read(
                    FLASH_START + LENGTH_BYTES,
                    config.as_mut_ptr().cast(),
                    length,
                )
            }
            .unwrap();

            serde_json::from_slice(&config).unwrap()
        };

        log::info!("config = {:?}", config);

        config
    }

    fn save(&self) {
        let encoded = serde_json::to_vec(self).unwrap();

        let mut data = encoded.len().to_ne_bytes().to_vec();
        data.extend_from_slice(&encoded);
        data.extend(iter::repeat(0).take(4 - (data.len() % 4)));

        let res = unsafe {
            esp_storage::ll::spiflash_unlock().unwrap();
            esp_storage::ll::spiflash_erase_sector(FLASH_START / FlashStorage::SECTOR_SIZE)
                .unwrap();
            esp_storage::ll::spiflash_write(FLASH_START, data.as_ptr().cast(), data.len() as u32)
        };
    }
}
