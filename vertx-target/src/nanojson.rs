use atoi::FromRadix10 as _;

pub(crate) trait Nanojson<'a>: Sized {
    fn parse(data: &'a [u8], offset: &mut usize) -> Option<Self>;
}

impl Nanojson<'_> for u8 {
    fn parse(data: &[u8], offset: &mut usize) -> Option<Self> {
        let (pin, len) = Self::from_radix_10(data);
        *offset += len;
        (len > 0).then_some(pin)
    }
}

impl<'a> Nanojson<'a> for &'a str {
    fn parse(data: &'a [u8], offset: &mut usize) -> Option<Self> {
        if data.get(*offset) != Some(&b'"') {
            return None;
        }

        *offset += 1;
        while data.get(*offset)? != &b'"' {
            *offset += 1;
        }
        *offset += 1;

        core::str::from_utf8(&data[1..*offset]).ok()
    }
}

impl<'a, T: Nanojson<'a>, const N: usize> Nanojson<'a> for heapless::Vec<T, N> {
    fn parse(data: &'a [u8], offset: &mut usize) -> Option<Self> {
        if data.get(*offset) != Some(&b'[') {
            return None;
        }
        *offset += 1;

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
            $($f:ident : $t:ty),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name {
            $($f: $t),+
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
                        $( stringify!($f) => $f = Some(<$t as crate::nanojson::Nanojson>::parse(data, offset)?), )+
                        _ => {}
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
