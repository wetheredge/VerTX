use atoi::FromRadix10Checked as _;

pub(crate) trait Nanojson<'a>: Sized {
    fn parse(data: &'a [u8], offset: &mut usize) -> Option<Self>;
}

impl Nanojson<'_> for u8 {
    fn parse(data: &[u8], offset: &mut usize) -> Option<Self> {
        let (pin, len) = Self::from_radix_10_checked(&data[*offset..]);
        *offset += len;
        (len > 0).then_some(pin?)
    }
}

impl<'a> Nanojson<'a> for &'a str {
    fn parse(data: &'a [u8], offset: &mut usize) -> Option<Self> {
        if data.get(*offset) != Some(&b'"') {
            return None;
        }
        *offset += 1;
        let start = *offset;

        loop {
            let &byte = data.get(*offset)?;
            if !byte.is_ascii() || byte.is_ascii_control() || byte == b'\\' {
                return None;
            } else if byte == b'"' {
                break;
            }

            *offset += 1;
        }

        let s = &data[start..*offset];

        // Closing quote
        *offset += 1;

        core::str::from_utf8(s).ok()
    }
}

impl<'a, T: Nanojson<'a>, const N: usize> Nanojson<'a> for heapless::Vec<T, N> {
    fn parse(data: &'a [u8], offset: &mut usize) -> Option<Self> {
        if data.get(*offset) != Some(&b'[') {
            return None;
        }
        *offset += 1;

        if data.get(*offset)? == &b']' {
            *offset += 1;
            return Some(Self::new());
        }

        let mut vec = Self::new();
        loop {
            skip_whitespace(data, offset);

            let item = T::parse(data, offset)?;
            vec.push(item).ok()?;

            skip_whitespace(data, offset);
            match data.get(*offset).copied() {
                Some(b',') => *offset += 1,
                Some(b']') => break,
                _ => return None,
            }
        }

        // Closing bracket
        *offset += 1;

        Some(vec)
    }
}

pub(crate) fn skip_whitespace(data: &[u8], offset: &mut usize) {
    while data.get(*offset).is_some_and(u8::is_ascii_whitespace) {
        *offset += 1;
    }
}

macro_rules! impl_nanojson {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident {
            $($fvis:vis $f:ident : $t:ty),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name {
            $($fvis $f: $t),+
        }

        impl crate::nanojson::Nanojson<'_> for $name {
            fn parse(data: &[u8], offset: &mut usize) -> Option<Self> {
                if data.get(*offset) != Some(&b'{') {
                    return None;
                }
                *offset += 1;

                $( let mut $f = None; )+

                loop {
                    crate::nanojson::skip_whitespace(data, offset);
                    let key = <&str>::parse(data, offset)?;
                    crate::nanojson::skip_whitespace(data, offset);

                    if data.get(*offset) != Some(&b':') {
                        return None;
                    }
                    *offset += 1;

                    crate::nanojson::skip_whitespace(data, offset);

                    match key {
                        $( stringify!($f) if $f.is_none() => $f = Some(<$t as crate::nanojson::Nanojson>::parse(data, offset)?), )+
                        _ => return None,
                    }

                    crate::nanojson::skip_whitespace(data, offset);
                    match data.get(*offset).copied() {
                        Some(b',') => *offset += 1,
                        Some(b'}') => break,
                        _ => return None,
                    }
                }

                // Closing brace
                *offset += 1;

                Some(Self {
                    $( $f: $f? ),+
                })
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use heapless::Vec;

    use super::*;

    fn parse_offset<'a, T: Nanojson<'a>>(data: &'a [u8], mut offset: usize) -> Option<(T, usize)> {
        let parsed = T::parse(data, &mut offset)?;
        Some((parsed, offset))
    }

    fn parse<'a, T: Nanojson<'a>>(data: &'a [u8]) -> Option<(T, usize)> {
        parse_offset(data, 0)
    }

    #[test]
    fn parse_u8() {
        assert_eq!(parse(b"0"), Some((0, 1)));
        assert_eq!(parse(b"255"), Some((255, 3)));
        assert_eq!(parse_offset(b" 0", 1), Some((0, 2)));

        assert_eq!(parse(b"0b1"), Some((0, 1)));
        assert_eq!(parse(b"0o1"), Some((0, 1)));
        assert_eq!(parse(b"0x1"), Some((0, 1)));
        assert_eq!(parse(b"0.1"), Some((0, 1)));
        assert_eq!(parse(b"0e1"), Some((0, 1)));
        assert_eq!(parse(b"0E1"), Some((0, 1)));
        assert_eq!(parse(b"0 "), Some((0, 1)));

        assert_eq!(parse::<u8>(b""), None);
        assert_eq!(parse::<u8>(b" 0"), None);
        assert_eq!(parse::<u8>(b"-1"), None);
        assert_eq!(parse::<u8>(b"+1"), None);
        assert_eq!(parse::<u8>(b"256"), None);
    }

    #[test]
    fn parse_str() {
        assert_eq!(parse(br#""""#), Some(("", 2)));
        assert_eq!(parse(br#"" ""#), Some((" ", 3)));
        assert_eq!(parse(br#""test""#), Some(("test", 6)));

        assert_eq!(parse_offset(br#" """#, 1), Some(("", 3)));
        assert_eq!(parse_offset(br#" " ""#, 1), Some((" ", 4)));

        assert_eq!(parse::<&str>(b""), None);
        assert_eq!(parse::<&str>(br#"""#), None);
        assert_eq!(parse::<&str>(br#"''"#), None);
        assert_eq!(parse::<&str>(br#" """#), None);
        assert_eq!(parse::<&str>(b"\"\n\""), None);
        assert_eq!(parse::<&str>(br#""\n""#), None);
    }

    #[test]
    fn parse_arrays() {
        fn vec<T: Clone, const N: usize>(slice: &[T]) -> Vec<T, N> {
            Vec::from_slice(slice).unwrap()
        }

        assert_eq!(parse(b"[]"), Some((vec::<u8, 0>(&[]), 2)));
        assert_eq!(parse(b"[0]"), Some((vec::<u8, 1>(&[0]), 3)));
        assert_eq!(
            parse(b"[ 0,1, 2, \n 3 ]"),
            Some((vec::<u8, 10>(&[0, 1, 2, 3]), 15))
        );
        assert_eq!(parse_offset(b" []", 1), Some((vec::<u8, 0>(&[]), 3)));

        assert_eq!(parse(br#"["test"]"#), Some((vec::<&str, 1>(&["test"]), 8)));

        assert_eq!(
            parse(b"[[]]"),
            Some((vec::<Vec<u8, 0>, 1>(&[Vec::new()]), 4))
        );

        assert_eq!(parse::<Vec<u8, 0>>(b" []"), None);
        assert_eq!(parse::<Vec<u8, 1>>(b"[0, 1]"), None);
        assert_eq!(parse::<Vec<u8, 2>>(br#"[0, "s"]"#), None);
        assert_eq!(parse::<Vec<&str, 2>>(br#"["s", 0]"#), None);
        assert_eq!(parse::<Vec<u8, 1>>(b"[0, , 1]"), None);
        assert_eq!(parse::<Vec<u8, 1>>(b"[0,]"), None);
    }

    #[test]
    fn parse_struct() {
        impl_nanojson! {
            #[derive(Debug, PartialEq, Eq)]
            struct Foo {
                foo: u8,
            }
        }

        assert_eq!(parse(br#"{"foo":1}"#), Some((Foo { foo: 1 }, 9)));
        assert_eq!(parse(br#"{ "foo" : 1 } "#), Some((Foo { foo: 1 }, 13)));
        assert_eq!(
            parse_offset(br#" { "foo" : 1 } "#, 1),
            Some((Foo { foo: 1 }, 14))
        );

        assert_eq!(parse::<Foo>(b"{}"), None);
        assert_eq!(parse::<Foo>(br#"{"foo"}"#), None);
        assert_eq!(parse::<Foo>(br#"{"foo":}"#), None);
        assert_eq!(parse::<Foo>(br#"{"foo":0,}"#), None);
        assert_eq!(parse::<Foo>(br#"{foo:0}"#), None);
        assert_eq!(parse::<Foo>(br#"{"foo":0,"foo":1}"#), None);
        assert_eq!(parse::<Foo>(br#"{"foo":0,"bar":0}"#), None);

        impl_nanojson! {
            #[derive(Debug, Default, PartialEq, Eq)]
            struct Bar {
                foo: u8,
                bar: Vec<u8, 2>,
            }
        }

        assert_eq!(
            parse(br#"{"foo":1,"bar":[0]}"#),
            Some((
                Bar {
                    foo: 1,
                    bar: Vec::from_slice(&[0]).unwrap()
                },
                19
            ))
        );
        assert_eq!(
            parse(b"{\n\t\"foo\": 0,\n\t\"bar\": []\n} "),
            Some((Bar::default(), 25))
        );
        assert_eq!(parse(br#"{"bar":[],"foo":0}"#), Some((Bar::default(), 18)));
        assert_eq!(
            parse_offset(br#" {"bar":[],"foo":0} "#, 1),
            Some((Bar::default(), 19))
        );

        assert_eq!(parse::<Bar>(br#"{"foo":0}"#), None);
        assert_eq!(parse::<Bar>(br#"{"bar":0}"#), None);
    }
}
