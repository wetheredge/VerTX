use core::fmt;

use heapless::{String, Vec};

pub(crate) type Id = u8;
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
        f: impl FnMut(Id, &str),
    ) -> Result<(), crate::storage::Error> {
        self.storage.names(f).await
    }

    pub(crate) async fn open(self, id: Id) -> Result<Model, crate::storage::Error> {
        let mut name = None;
        self.storage
            .names(|i, n: &str| {
                if i == id {
                    name = Some(name_from_str(n));
                }
            })
            .await?;
        let name = loog::unwrap!(name, "No name for model {id=u8}");

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

fn name_from_str(raw_name: &str) -> Name {
    let len = raw_name.len().min(NAME_LEN);
    let name = &raw_name.as_bytes()[0..len];
    let name = Vec::from_slice(name).unwrap();
    let Ok(name) = String::from_utf8(name) else {
        loog::panic!("Name wasn't ascii: {raw_name=str:?}");
    };
    name
}
