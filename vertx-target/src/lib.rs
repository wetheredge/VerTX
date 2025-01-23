#![no_std]

#[macro_use]
mod nanojson;

use heapless::Vec;
use serde::Serialize;

use crate::nanojson::Nanojson;

pub type Pin = u8;

impl_nanojson! {
    #[derive(Debug, Serialize)]
    pub struct Target {
        status_led: Pin,
        display: Display,
        ui: Ui,
        inputs: Inputs,
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Display {
    Ssd1306 { sda: Pin, scl: Pin },
}

impl_nanojson! {
    #[derive(Debug, Serialize)]
    pub struct Ui {
        up: Pin,
        down: Pin,
        left: Pin,
        right: Pin,
    }
}

impl_nanojson! {
    #[derive(Debug, Serialize)]
    pub struct Inputs {
        analog: Vec<Pin, 16>,
        digital: Vec<Pin, 16>,
    }
}

impl Target {
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        Nanojson::parse(data, &mut 0)
    }
}

impl Nanojson<'_> for Display {
    fn parse(data: &[u8], offset: &mut usize) -> Option<Self> {
        if data.get(*offset) != Some(&b'{') {
            return None;
        }
        *offset += 1;

        let mut typ = None;
        let mut sda = None;
        let mut scl = None;

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
                "type" => typ = Some(Nanojson::parse(data, offset)?),
                "sda" => sda = Some(Nanojson::parse(data, offset)?),
                "scl" => scl = Some(Nanojson::parse(data, offset)?),
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

        Some(match typ {
            Some("ssd1306") => Self::Ssd1306 {
                sda: sda?,
                scl: scl?,
            },
            _ => return None,
        })
    }
}
