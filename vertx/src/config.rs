#![allow(clippy::host_endian_bytes)]
#![warn(clippy::big_endian_bytes, clippy::little_endian_bytes)]

use alloc::vec;

use embassy_sync::mutex::Mutex;
use portable_atomic::{AtomicBool, Ordering};
use vertx_config::update;

use crate::hal::traits::ConfigStorage as _;
use crate::hal::ConfigStorage;

pub struct Manager {
    modified: AtomicBool,
    storage: Mutex<crate::mutex::SingleCore, ConfigStorage>,
    config: crate::Config,
}

impl Manager {
    pub fn new(storage: ConfigStorage) -> Self {
        let config = match storage.load(vertx_config::storage::postcard::from_slice) {
            Some(Ok(config)) => config,
            Some(Err(err)) => {
                log::error!("Failed to load config: {err}");
                Default::default()
            }
            None => Default::default(),
        };

        Self {
            modified: AtomicBool::new(false),
            storage: Mutex::new(storage),
            config,
        }
    }

    pub async fn save(&self) {
        if !self.modified.swap(false, Ordering::AcqRel) {
            return;
        }

        log::info!("Writing configuration");

        let encoded = vertx_config::storage::postcard::to_vec(&self.config).await;

        let mut data = vec![0; 1 + encoded.len().div_ceil(4)];
        data[0] = encoded.len() as u32;
        bytemuck::cast_slice_mut(&mut data)[4..(4 + encoded.len())].copy_from_slice(&encoded);

        let mut storage = self.storage.lock().await;
        storage.save(&data);
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
