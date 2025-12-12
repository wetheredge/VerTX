use core::fmt;

use heapless::{String, Vec};

use crate::hal::prelude::*;
use crate::storage::File;

pub(crate) type Id = u16;
const NAME_LEN: usize = 16;
pub(crate) type Name = String<NAME_LEN>;

#[derive(Clone, Copy)]
pub(crate) struct Manager {
    storage: crate::storage::Models,
}

impl Manager {
    pub(crate) fn new(storage: crate::storage::Models) -> Self {
        Self { storage }
    }

    pub(crate) async fn for_each_name(
        self,
        mut f: impl FnMut(Id, Name),
    ) -> Result<(), crate::storage::Error> {
        self.storage
            .for_each(async |id, file| {
                f(id, read_name(file).await?);
                Ok(())
            })
            .await
    }

    pub(crate) async fn open(self, id: Id) -> Result<Model, crate::storage::Error> {
        let Some(name) = self.storage.model(id, read_name).await? else {
            loog::panic!("Model {id=u16} doesn't exist");
        };
        Ok(Model {
            storage: self.storage,
            id,
            name,
        })
    }
}

impl fmt::Debug for Manager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Manager").finish_non_exhaustive()
    }
}

pub(crate) struct Model {
    #[expect(unused)]
    storage: crate::storage::Models,
    id: Id,
    name: Name,
}

impl Model {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Model")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish_non_exhaustive()
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
