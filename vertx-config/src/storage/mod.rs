pub mod postcard;

use alloc::vec::Vec;
use core::future::Future;

pub trait Storage {
    fn save<S: Serializer>(&self, serializer: S) -> impl Future<Output = ()>;
    fn load(from: Stored<'_>) -> Self;
}

pub trait Serializer {
    type StructSerializer: StructSerializer;

    fn boolean(self, b: bool);
    fn string(self, s: &str);
    fn unsigned(self, u: u32);
    fn signed(self, i: i32);
    fn float(self, f: f32);
    fn structure(self, fields: usize) -> Self::StructSerializer;
}

pub trait StructSerializer {
    fn field<V: Storage>(&mut self, name: &str, value: &V) -> impl Future<Output = ()>;
    fn finish(self);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Stored<'a> {
    Boolean(bool),
    String(&'a str),
    Unsigned(u32),
    Signed(i32),
    Float(f32),
    Struct(#[serde(borrow)] Vec<(&'a str, Stored<'a>)>),
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;

    use super::*;

    #[tokio::test]
    async fn settings() {
        #[derive(Debug, PartialEq, Eq, crate::Storage)]
        struct Test {
            loaded: bool,
            default: String,
        }

        impl Default for Test {
            fn default() -> Self {
                Self {
                    loaded: false,
                    default: String::from("test"),
                }
            }
        }

        let expected = Test {
            loaded: true,
            default: String::from("test"),
        };
        let got = Test::load(Stored::Struct(vec![("loaded", Stored::Boolean(true))]));
        assert_eq!(expected, got);
    }
}
