use core::cell::RefCell;
use core::fmt;
use core::mem::MaybeUninit;

use embassy_sync::blocking_mutex::Mutex;
use heapless::{String, Vec};
use portable_atomic::{AtomicBool, Ordering};
use static_cell::StaticCell;

use crate::hal::prelude::*;
use crate::storage::{Directory, Error as StorageError, File};

const RAW_NAME_LEN: usize = 4;
const NAME_LEN: usize = 16;
pub(crate) type Name = String<NAME_LEN>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RawName {
    bytes: [u8; RAW_NAME_LEN],
}

impl RawName {
    fn from_bytes(raw: &[u8]) -> Option<Self> {
        if raw.len() != RAW_NAME_LEN {
            loog::error!(
                "Model filename length is incorrect: {} != {RAW_NAME_LEN}",
                raw.len()
            );
            return None;
        }

        if core::str::from_utf8(raw).is_err() {
            loog::error!("Invalid model filename: {raw=[u8]:?}");
            return None;
        }

        let mut bytes = [0; RAW_NAME_LEN];
        bytes.copy_from_slice(raw);
        Some(Self { bytes })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Manager {
    init: &'static AtomicBool,
    state: &'static Mutex<crate::mutex::MultiCore, RefCell<MaybeUninit<State>>>,
}

struct State {
    dir: Directory,
}

impl Manager {
    pub(crate) fn new() -> Self {
        static INIT: AtomicBool = AtomicBool::new(false);
        static STATE: StaticCell<Mutex<crate::mutex::MultiCore, RefCell<MaybeUninit<State>>>> =
            StaticCell::new();

        let state = STATE.init_with(|| Mutex::new(RefCell::new(MaybeUninit::uninit())));

        Self { init: &INIT, state }
    }

    pub(crate) fn init(self, dir: Directory) {
        if self.init.load(Ordering::Relaxed) {
            loog::warn!("models::Manager::init() called multiple times");
            return;
        }

        self.state
            .lock(|state| state.replace(MaybeUninit::new(State { dir })));

        self.init.store(true, Ordering::Relaxed);
    }

    pub(crate) async fn for_each(
        self,
        mut callback: impl FnMut(RawName, Name),
    ) -> Result<(), StorageError> {
        let mut entries = self.state(|state| state.dir.iter()).await;

        while let Some(entry) = loog::unwrap!(entries.next().await.transpose()) {
            if !entry.is_file() {
                continue;
            }

            let Some(raw_name) = RawName::from_bytes(entry.name()) else {
                continue;
            };

            let Some(mut file) = entry.to_file() else {
                loog::unreachable!("already checked by is_file()");
            };

            let name = loog::unwrap!(read_name(&mut file).await);
            loog::unwrap!(file.close().await);

            callback(raw_name, name);
        }

        Ok(())
    }

    pub(crate) async fn open(self, raw: RawName) -> Result<Model, crate::storage::Error> {
        let raw = core::str::from_utf8(&raw.bytes).unwrap();
        let dir = self.state(|state| state.dir.clone()).await;
        let mut file = dir.file(raw).await?;
        let name = read_name(&mut file).await?;
        Ok(Model { name })
    }

    /// Busy-wait until initialized
    async fn wait_for_init(self) {
        while !self.init.load(Ordering::Relaxed) {
            embassy_futures::yield_now().await;
        }
    }

    async fn state<T>(self, f: impl FnOnce(&State) -> T) -> T {
        self.wait_for_init().await;
        self.state.lock(|state| {
            let state = state.borrow();
            // SAFETY: guaranteed to be initialized after self.wait_for_init() returns
            let state = unsafe { state.assume_init_ref() };
            f(state)
        })
    }
}

impl fmt::Debug for Manager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Manager").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) struct Model {
    name: Name,
}

impl Model {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

async fn read_name(file: &mut File) -> Result<Name, crate::storage::Error> {
    let mut len = [0; 1];
    file.read(&mut len).await?;
    let mut len = len[0] as usize;

    if len > NAME_LEN {
        loog::warn!("Truncating model name from {len} to {NAME_LEN}");
        len = NAME_LEN;
    }

    let mut buffer = Vec::new();
    loog::unwrap!(buffer.resize_default(len));
    loog::unwrap!(file.read_exact(&mut buffer).await);
    Ok(String::from_utf8(buffer).unwrap())
}
