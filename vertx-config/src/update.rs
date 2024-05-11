use core::future::Future;

pub trait UpdateMut {
    fn update_mut<'a>(&mut self, key: &'a str, update: Update<'a>) -> impl Future<Output = Result>;
}

pub trait UpdateRef {
    fn update_ref<'a>(&self, key: &'a str, update: Update<'a>) -> impl Future<Output = Result>;
}

impl<T: UpdateRef> UpdateMut for T {
    fn update_mut<'a>(&mut self, key: &'a str, update: Update<'a>) -> impl Future<Output = Result> {
        self.update_ref(key, update)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum Update<'a> {
    Boolean(bool),
    String(&'a str),
    Unsigned(u32),
    Signed(i32),
    Float(f32),
}

pub type Result = core::result::Result<(), Error>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Error {
    KeyNotFound,
    InvalidType,
    InvalidValue,
    TooSmall { min: i64 },
    TooLarge { max: i64 },
}
