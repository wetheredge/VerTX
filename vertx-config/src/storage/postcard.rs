#![allow(missing_debug_implementations, clippy::little_endian_bytes)]
#![deny(clippy::host_endian_bytes, clippy::big_endian_bytes)]

use alloc::vec::Vec;

use num_enum::{IntoPrimitive, TryFromPrimitive};

use super::Storage;

pub async fn to_vec<T: Storage>(config: &T) -> Vec<u8> {
    let mut buffer = Vec::new();
    config.save(&mut Postcard(&mut buffer)).await;
    buffer
}

pub fn from_slice<T: Storage + Default>(data: &[u8]) -> ::postcard::Result<T> {
    ::postcard::from_bytes(data).map(T::load)
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum Kind {
    Boolean,
    String,
    Unsigned,
    Signed,
    Float,
    Struct,
}

pub struct Postcard<'a>(&'a mut Vec<u8>);

impl Postcard<'_> {
    fn write_varuint(&mut self, mut x: usize) {
        while x > 0x7F {
            self.0.push(0x80 | (x as u8 & 0x7F));
            x >>= 7;
        }
        self.0.push(x as u8);
    }

    fn write_varint(&mut self, x: isize) {
        self.write_varuint(((x << 1) ^ (x >> (isize::BITS - 1))) as usize);
    }

    fn write_str(&mut self, s: &str) {
        self.write_varuint(s.len());
        self.0.extend_from_slice(s.as_bytes());
    }
}

impl super::Serializer for &mut Postcard<'_> {
    type StructSerializer = Self;

    fn boolean(self, b: bool) {
        self.0.push(Kind::Boolean.into());
        self.0.push(u8::from(b).to_le());
    }

    fn string(self, s: &str) {
        self.0.push(Kind::String.into());
        self.write_str(s);
    }

    fn unsigned(self, u: u32) {
        self.0.push(Kind::Unsigned.into());
        self.write_varuint(u as usize);
    }

    fn signed(self, i: i32) {
        self.0.push(Kind::Signed.into());
        self.write_varint(i as isize);
    }

    fn float(self, f: f32) {
        self.0.push(Kind::Float.into());
        self.0.extend_from_slice(&f.to_le_bytes());
    }

    fn structure(self, fields: usize) -> Self::StructSerializer {
        self.0.push(Kind::Struct.into());
        self.write_varuint(fields);
        self
    }
}

impl super::StructSerializer for &mut Postcard<'_> {
    async fn field<V: Storage>(&mut self, name: &str, value: &V) {
        self.write_str(name);
        value.save(&mut Postcard(self.0)).await;
    }

    fn finish(self) {}
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use core::f32;

    use super::*;
    use crate::storage::Stored;

    macro_rules! test_struct {
        ($( $f:ident : $t:ty = $v:expr ),+) => {
            #[derive(Debug, Default, crate::Storage)]
            struct Test {
                $( $f: $t ),+
            }

            impl Test {
                fn new() -> Self {
                    Self {
                        $( $f: $v ),+
                    }
                }
            }
        };
    }

    test_struct! {
        bool_false: bool = false,
        bool_true: bool = true,
        string_empty: String = String::new(),
        string: String = String::from("test"),
        u8_0: u8 = 0,
        u8_max: u8 = u8::MAX,
        i8_0: i8 = 0,
        i8_neg: i8 = -3,
        i8_max: i8 = i8::MAX,
        u16_0: u16 = 0,
        u16_small: u16 = 0x101,
        u16_max: u16 = u16::MAX,
        i16_0: i16 = 0,
        i16_neg: i16 = -3,
        i16_max: i16 = i16::MAX,
        u32_0: u32 = 0,
        u32_small: u32 = 0x101,
        u32_max: u32 = u32::MAX,
        i32_0: i32 = 0,
        i32_neg: i32 = -3,
        i32_max: i32 = i32::MAX,
        f32_0: f32 = 0.0,
        f32_neg: f32 = -f32::consts::PI,
        f32_pos: f32 = f32::consts::PI,
        nested: Nested = Nested { inner: true },
        boot_snapshot: crate::BootSnapshot<bool, embassy_sync::blocking_mutex::raw::NoopRawMutex> = crate::BootSnapshot::from(true),
        reactive: crate::Reactive<bool, embassy_sync::blocking_mutex::raw::NoopRawMutex> = crate::Reactive::from(true)
    }

    #[derive(Debug, Default, crate::Storage)]
    struct Nested {
        inner: bool,
    }

    fn encoded() -> Stored<'static> {
        Stored::Struct(vec![
            ("bool_false", Stored::Boolean(false)),
            ("bool_true", Stored::Boolean(true)),
            ("string_empty", Stored::String("")),
            ("string", Stored::String("test")),
            ("u8_0", Stored::Unsigned(0)),
            ("u8_max", Stored::Unsigned(u8::MAX.into())),
            ("i8_0", Stored::Signed(0)),
            ("i8_neg", Stored::Signed(-3)),
            ("i8_max", Stored::Signed(i8::MAX.into())),
            ("u16_0", Stored::Unsigned(0)),
            ("u16_small", Stored::Unsigned(0x101)),
            ("u16_max", Stored::Unsigned(u16::MAX.into())),
            ("i16_0", Stored::Signed(0)),
            ("i16_neg", Stored::Signed(-3)),
            ("i16_max", Stored::Signed(i16::MAX.into())),
            ("u32_0", Stored::Unsigned(0)),
            ("u32_small", Stored::Unsigned(0x101)),
            ("u32_max", Stored::Unsigned(u32::MAX)),
            ("i32_0", Stored::Signed(0)),
            ("i32_neg", Stored::Signed(-3)),
            ("i32_max", Stored::Signed(i32::MAX)),
            ("f32_0", Stored::Float(0.0)),
            ("f32_neg", Stored::Float(-f32::consts::PI)),
            ("f32_pos", Stored::Float(f32::consts::PI)),
            (
                "nested",
                Stored::Struct(vec![("inner", Stored::Boolean(true))]),
            ),
            ("boot_snapshot", Stored::Boolean(true)),
            ("reactive", Stored::Boolean(true)),
        ])
    }

    #[tokio::test]
    async fn serialize() {
        assert_eq!(
            ::postcard::to_allocvec(&encoded()).unwrap(),
            to_vec(&Test::new()).await
        );
    }

    #[tokio::test]
    async fn round_trip() {
        let bytes = ::postcard::to_allocvec(&encoded()).unwrap();
        let deserialized: Test = from_slice(&bytes).unwrap();
        assert_eq!(bytes, to_vec(&deserialized).await);
    }
}
