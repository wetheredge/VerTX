#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod decoder;
mod encoder;

use alloc::ffi::CString;
use alloc::vec;
use core::fmt;

use crc::Crc;

pub use self::decoder::DecodeError;
use self::decoder::Decoder;
pub use self::encoder::EncodeError;
use self::encoder::Encoder;

// TODO: benchmark lookup table options
const BASIC_CRC: Crc<u8> = Crc::<u8>::new(&crc::Algorithm {
    width: 8,
    poly: 0xD5,
    init: 0x00,
    refin: false,
    refout: false,
    xorout: 0x00,
    check: 0x00,
    residue: 0x00,
});

macro_rules! enum_repr {
    (
        #[repr($repr:ty)]
        $(#[$attr:meta])*
        $pub:vis enum $name:ident {
            $( $( #[$variant_attr:meta] )* $variant:ident = $value:expr ),*
            $(,)?
        }
    ) => {
        $(#[$attr])*
        #[repr($repr)]
        $pub enum $name {
            $( $(#[$variant_attr])* $variant = $value ),*
        }

        impl $name {
            fn from_raw(raw: $repr) -> Option<Self> {
                match raw {
                    $( $value => Some(Self::$variant), )*
                    _ => None,
                }
            }

            fn into_raw(self) -> $repr {
                match self {
                    $( Self::$variant => $value, )*
                }
            }
        }
    }
}

enum_repr! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Address {
        Broadcast = 0x00,
        Usb = 0x10,
        /// Bluetooth module
        Bluetooth = 0x12,
        TbsCorePnpPro = 0x80,
        Reserved1 = 0x8A,
        /// External current sensor
        CurrentSensor = 0xC0,
        /// External GPS
        Gps = 0xC2,
        /// External blackbox logging device
        TbsBlackbox = 0xC4,
        /// Flight Controller (Betaflight / iNav)
        FlightController = 0xC8,
        Reserved2 = 0xCA,
        RaceTag = 0xCC,
        /// `CRSF_ADDRESS_RADIO_TRANSMITTER`
        Handset = 0xEA,
        /// Radio receiver (`CRSF_ADDRESS_CRSF_RECEIVER`)
        Receiver = 0xEC,
        /// Radio transmitter module (`CRSF_ADDRESS_CRSF_TRANSMITTER`)
        Transmitter = 0xEE,
        /// **Non-standard** source address used by ExpressLRS Lua
        ElrsLua = 0xEF,
    }
}

impl Address {
    fn from_u16(raw: u16) -> Option<Self> {
        if raw > u16::from(u8::MAX) {
            None
        } else {
            Self::from_raw(raw as u8)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Packet {
    /// GPS position, ground speed, heading, altitude, satellite count
    Gps {
        /// Latitude in degrees (7 decimals)
        latitude: i32,
        /// Longitude in degrees (7 decimals)
        longitude: i32,
        /// Ground speed in km/h (1 decimal)
        speed: i16,
        /// Heading in degrees (2 decimals)
        heading: i16,
        /// Altitude in meters
        altitude: i32,
        /// Satellite count
        satellites: u8,
    },
    /// Vertical speed
    Vario {
        /// Vertical speed in cm/s
        vertical_speed: i16,
    },
    /// Battery voltage, current, mAh, remaining percent
    BatterySensor {
        /// Battery voltage in decivolts
        voltage: i16,
        /// Current in deciamps
        current: i16,
        /// Estimated used capacity in mAh
        used: i32,
        /// Estimated remaining battery in whole percentage points
        remaining: i8,
    },
    /// Barometric altitude, vertical speed (optional)
    BarometricAltitude {
        /// Barometric altitude in decimeters
        altitude: i32,
        /// Vertical speed in cm/s
        vertical_speed: Option<i16>,
    },
    /// Heartbeat **(CRSFv3 only)**
    Heartbeat {
        origin: Address,
    },
    /// Signal information. uplink/downlink RSSI, SNR, link quality (LQ), RF
    /// mode, transmit power
    LinkStatistics {
        /// RX→TX RSSI for antenna 1 in negative dBm
        up_rssi1: u8,
        /// RX→TX RSSI for antenna 2 in negative dBm
        up_rssi2: u8,
        /// RX→TX link quality in whole percentage points
        up_lq: u8,
        /// RX→TX signal to noise ratio in dB
        up_snr: i8,
        /// Active antenna for RX antenna diversity
        active_antenna: u8,
        /// Link dependent mode / packet rate
        mode: u8,
        tx_power: TxPower,
        /// TX→RX RSSI in negative dBm
        down_rssi: u8,
        /// TX→RX link quality in whole percentage points
        down_lq: u8,
        /// TX→RX signal to noise ratio in dB
        down_snr: i8,
    },
    /// Channel data (both handset to TX and RX to flight controller)
    RcChannelsPacked(RcChannelsPacked),
    /// Channels subset data **(CRSFv3 only)**
    SubsetRcChannelsPacked,
    /// Receiver RSSI percent, power?
    LinkRxId,
    /// Transmitter RSSI percent, power, fps?
    LinkTxId,
    /// Attitude: pitch, roll, yaw
    Attitude,
    /// Flight controller flight mode string
    FlightMode(CString),
    /// Sender requesting `DeviceInfo` from all destination devices
    DevicePing {
        to: Address,
        from: Address,
    },
    /// Device name, firmware version, hardware version, serial number (`Ping`
    /// response)
    DeviceInfo {
        to: Address,
        from: Address,
        name: CString,
        serial: u32,
        hardware_version: u32,
        software_version: u32,
        config_parameters: u8,
        config_protocol: u8,
    },
    /// Configuration item data chunk
    ParameterSettingsEntry,
    /// Configuration item read request
    ParameterRead,
    /// Configuration item write request
    ParameterWrite,
    /// **Non-standard** ExpressLRS good/bad packet count, status flags
    ElrsStatus,
    /// **CRSF** command execute
    Command,
    /// Extended type used for OPENTX_SYNC
    RadioId,
    KissRequest,
    KissResponse,
    /// MSP parameter request / command
    MspRequest,
    /// MSP parameter response chunk
    MspResponse,
    /// MSP parameter write
    MspWrite,
    /// MSP DisplayPort control command **(CRSFv3 only)**
    DisplayportCommand,
    /// Ardupilot output?
    ArdupilotResponse,
}

enum_repr! {
    #[repr(u8)]
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[allow(non_camel_case_types)]
    pub enum TxPower {
        mW_0 = 0,
        mW_10 = 1,
        mW_25 = 2,
        mW_50 = 7,
        mW_100 = 3,
        mW_500 = 4,
        mW_1000 = 5,
        mW_2000 = 6,
    }
}

impl fmt::Debug for TxPower {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::mW_0 => "0mW",
            Self::mW_10 => "10mW",
            Self::mW_25 => "25mW",
            Self::mW_50 => "50mW",
            Self::mW_100 => "100mW",
            Self::mW_500 => "500mW",
            Self::mW_1000 => "1000mW",
            Self::mW_2000 => "2000mW",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RcChannelsPacked(RcChannelsPackedInner<[u8; 22]>);

bitfield::bitfield! {
    #[derive(Clone, PartialEq)]
    struct RcChannelsPackedInner([u8]);
    impl Debug;
    u16;
    channel0, set_channel0: 10, 0;
    channel1, set_channel1: 21, 11;
    channel2, set_channel2: 32, 22;
    channel3, set_channel3: 43, 33;
    channel4, set_channel4: 54, 44;
    channel5, set_channel5: 65, 55;
    channel6, set_channel6: 76, 66;
    channel7, set_channel7: 87, 77;
    channel8, set_channel8: 98, 88;
    channel9, set_channel9: 109, 99;
    channel10, set_channel10: 120, 110;
    channel11, set_channel11: 131, 121;
    channel12, set_channel12: 142, 132;
    channel13, set_channel13: 153, 143;
    channel14, set_channel14: 164, 154;
    channel15, set_channel15: 175, 165;
}

macro_rules! impl_rc_channels_packed {
    ($($channel:ident),*) => {
        pub fn unpack(&self) -> [u16; 16] {
            [$(self.$channel()),*]
        }

        $(
            #[inline(always)]
            pub fn $channel(&self) -> u16 {
                self.0.$channel()
            }
        )*
};
}

impl RcChannelsPacked {
    impl_rc_channels_packed!(
        channel0, channel1, channel2, channel3, channel4, channel5, channel6, channel7, channel8,
        channel9, channel10, channel11, channel12, channel13, channel14, channel15
    );

    pub fn new(channels: [u16; 16]) -> Self {
        let mut inner = RcChannelsPackedInner([0; 22]);
        inner.set_channel0(channels[0]);
        inner.set_channel1(channels[1]);
        inner.set_channel2(channels[2]);
        inner.set_channel3(channels[3]);
        inner.set_channel4(channels[4]);
        inner.set_channel5(channels[5]);
        inner.set_channel6(channels[6]);
        inner.set_channel7(channels[7]);
        inner.set_channel8(channels[8]);
        inner.set_channel9(channels[9]);
        inner.set_channel10(channels[10]);
        inner.set_channel11(channels[11]);
        inner.set_channel12(channels[12]);
        inner.set_channel13(channels[13]);
        inner.set_channel14(channels[14]);
        inner.set_channel15(channels[15]);
        Self(inner)
    }
}

impl Packet {
    #[allow(clippy::match_same_arms)]
    pub fn decode(raw: &[u8]) -> Result<(Self, usize), DecodeError> {
        let mut decoder = Decoder::new(raw)?;

        let packet = if decoder.packet_type() < 0x28 {
            match decoder.packet_type() {
                // CRSF_FRAMETYPE_GPS
                0x02 => Self::Gps {
                    latitude: decoder.i32(),
                    longitude: decoder.i32(),
                    speed: decoder.i16(),
                    heading: decoder.i16(),
                    altitude: i32::from(decoder.u16()) - 1000,
                    satellites: decoder.u8(),
                },
                // CRSF_FRAMETYPE_VARIO
                0x07 => Self::Vario {
                    vertical_speed: decoder.i16(),
                },
                // CRSF_FRAMETYPE_BATTERY_SENSOR
                0x08 => Self::BatterySensor {
                    voltage: decoder.i16(),
                    current: decoder.i16(),
                    used: decoder.i24(),
                    remaining: decoder.i8(),
                },
                // CRSF_FRAMETYPE_BARO_ALTITUDE
                0x09 => Self::BarometricAltitude {
                    altitude: convert_barometric_altitude(decoder.i16()),
                    vertical_speed: (decoder.payload_len() == 2).then(|| decoder.i16()),
                },
                // CRSF_FRAMETYPE_HEARTBEAT
                0x0B => {
                    // FIXME
                    let origin = Address::from_u16(decoder.u16()).unwrap();

                    Self::Heartbeat { origin }
                }
                // CRSF_FRAMETYPE_LINK_STATISTICS
                0x14 => {
                    Self::LinkStatistics {
                        up_rssi1: decoder.u8(),
                        up_rssi2: decoder.u8(),
                        up_lq: decoder.u8(),
                        up_snr: decoder.i8(),
                        active_antenna: decoder.u8(),
                        mode: decoder.u8(),
                        // FIXME
                        tx_power: TxPower::from_raw(decoder.u8()).unwrap(),
                        down_rssi: decoder.u8(),
                        down_lq: decoder.u8(),
                        down_snr: decoder.i8(),
                    }
                }
                // CRSF_FRAMETYPE_RC_CHANNELS_PACKED
                0x16 => {
                    // FIXME
                    let mut channels = [0; 22];
                    channels.clone_from_slice(decoder.payload());
                    Self::RcChannelsPacked(RcChannelsPacked(RcChannelsPackedInner(channels)))
                }
                // CRSF_FRAMETYPE_SUBSET_RC_CHANNELS_PACKED
                0x17 => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_LINK_RX_ID
                0x1C => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_LINK_TX_ID
                0x1D => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_ATTITUDE
                0x1E => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_FLIGHT_MODE
                0x21 => {
                    let length = decoder.payload_len();
                    // FIXME
                    let mut data = vec![0; length];
                    data.clone_from_slice(decoder.payload());
                    // Strip trailing 0
                    data.truncate(length - 1);

                    // FIXME: return error instead of unwrapping?
                    let mode = CString::new(data).unwrap();
                    Self::FlightMode(mode)
                }

                kind => return Err(DecodeError::InvalidPacketKind(kind)),
            }
        } else {
            let mut decoder = decoder.extended()?;

            match decoder.packet_type() {
                // CRSF_FRAMETYPE_DEVICE_PING
                0x28 => Self::DevicePing {
                    to: decoder.to(),
                    from: decoder.from(),
                },
                // CRSF_FRAMETYPE_DEVICE_INFO
                0x29 => Self::DeviceInfo {
                    to: decoder.to(),
                    from: decoder.from(),
                    name: decoder.string(),
                    serial: decoder.u32(),
                    hardware_version: decoder.u32(),
                    software_version: decoder.u32(),
                    config_parameters: decoder.u8(),
                    config_protocol: decoder.u8(),
                },
                // CRSF_FRAMETYPE_PARAMETER_SETTINGS_ENTRY
                0x2B => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_PARAMETER_READ
                0x2C => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_PARAMETER_WRITE
                0x2D => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_ELRS_STATUS
                0x2E => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_COMMAND
                0x32 => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_RADIO_ID
                0x3A => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_KISS_REQ
                0x78 => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_KISS_RESP
                0x79 => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_MSP_REQ
                0x7A => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_MSP_RESP
                0x7B => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_MSP_WRITE
                0x7C => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_DISPLAYPORT_CMD
                0x7D => return Err(DecodeError::UnsupportedPacket),
                // CRSF_FRAMETYPE_ARDUPILOT_RESP
                0x80 => return Err(DecodeError::UnsupportedPacket),

                kind => return Err(DecodeError::InvalidPacketKind(kind)),
            }
        };

        Ok((packet, decoder.len()))
    }

    #[allow(clippy::match_same_arms)]
    pub fn encode(&self, buffer: &mut [u8]) -> Result<usize, EncodeError> {
        let mut encoder = Encoder::new(buffer)?;

        match self {
            Self::Gps { .. } => Err(EncodeError::UnsupportedPacket),
            Self::Vario { .. } => Err(EncodeError::UnsupportedPacket),
            Self::BatterySensor {
                voltage,
                current,
                used,
                remaining,
            } => {
                encoder.i16(*voltage)?;
                encoder.i16(*current)?;
                encoder.i24(*used)?;
                encoder.i8(*remaining)?;
                encoder.finish(0x08)
            }
            Self::BarometricAltitude { .. } => Err(EncodeError::UnsupportedPacket),
            Self::Heartbeat { origin } => {
                encoder.u8(origin.into_raw())?;
                encoder.finish(0x0B)
            }
            Self::LinkStatistics {
                up_rssi1,
                up_rssi2,
                up_lq,
                up_snr,
                active_antenna,
                mode,
                tx_power,
                down_rssi,
                down_lq,
                down_snr,
            } => {
                encoder.u8(*up_rssi1)?;
                encoder.u8(*up_rssi2)?;
                encoder.u8(*up_lq)?;
                encoder.i8(*up_snr)?;
                encoder.u8(*active_antenna)?;
                encoder.u8(*mode)?;
                encoder.u8(tx_power.into_raw())?;
                encoder.u8(*down_rssi)?;
                encoder.u8(*down_lq)?;
                encoder.i8(*down_snr)?;
                encoder.finish(0x14)
            }
            Self::RcChannelsPacked(channels) => {
                encoder.slice(&(channels.0).0)?;
                encoder.finish(0x18)
            }
            Self::SubsetRcChannelsPacked => Err(EncodeError::UnsupportedPacket),
            Self::LinkRxId => Err(EncodeError::UnsupportedPacket),
            Self::LinkTxId => Err(EncodeError::UnsupportedPacket),
            Self::Attitude => Err(EncodeError::UnsupportedPacket),
            Self::FlightMode(mode) => {
                encoder.string(mode)?;
                encoder.finish(0x21)
            }
            Self::DevicePing { to, from } => {
                encoder.extended(*to, *from)?;
                encoder.finish(0x28)
            }
            Self::DeviceInfo {
                to,
                from,
                name,
                serial,
                hardware_version,
                software_version,
                config_parameters,
                config_protocol,
            } => {
                encoder.extended(*to, *from)?;
                encoder.string(name)?;
                encoder.u32(*serial)?;
                encoder.u32(*hardware_version)?;
                encoder.u32(*software_version)?;
                encoder.u8(*config_parameters)?;
                encoder.u8(*config_protocol)?;
                encoder.finish(0x29)
            }
            Self::ParameterSettingsEntry => Err(EncodeError::UnsupportedPacket),
            Self::ParameterRead => Err(EncodeError::UnsupportedPacket),
            Self::ParameterWrite => Err(EncodeError::UnsupportedPacket),
            Self::ElrsStatus => Err(EncodeError::UnsupportedPacket),
            Self::Command => Err(EncodeError::UnsupportedPacket),
            Self::RadioId => Err(EncodeError::UnsupportedPacket),
            Self::KissRequest => Err(EncodeError::UnsupportedPacket),
            Self::KissResponse => Err(EncodeError::UnsupportedPacket),
            Self::MspRequest => Err(EncodeError::UnsupportedPacket),
            Self::MspResponse => Err(EncodeError::UnsupportedPacket),
            Self::MspWrite => Err(EncodeError::UnsupportedPacket),
            Self::DisplayportCommand => Err(EncodeError::UnsupportedPacket),
            Self::ArdupilotResponse => Err(EncodeError::UnsupportedPacket),
        }
    }
}

fn convert_barometric_altitude(altitude: i16) -> i32 {
    if altitude.is_negative() {
        i32::from(altitude & 0x7FFF) * 10
    } else {
        i32::from(altitude) - 10_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_pair {
        (enc: $encode:ident,dec: $decode:ident,packet: $packet:expr,raw: $raw:expr,) => {
            #[test]
            fn $encode() {
                let expected: &[u8] = $raw;
                let mut encoded = vec![0; expected.len()];
                $packet.encode(&mut encoded).unwrap();
                assert_eq!(expected, encoded);
            }

            #[test]
            fn $decode() {
                let raw: &[u8] = $raw;
                let packet = Packet::decode(raw).unwrap();
                assert_eq!(($packet, raw.len()), packet);
            }
        };
    }

    #[test]
    fn decode_rc_channels_packed() {
        let raw: &[u8] = &[
            0xC8, 0x18, 0x16, 0xE3, 0xEB, 0x5E, 0x2B, 0xC8, 0xD7, 0x8A, 0x56, 0xB4, 0x02, 0x7C,
            0xE0, 0x03, 0x1F, 0xF8, 0xC0, 0x07, 0x3E, 0xF0, 0x81, 0x0F, 0x7C, 0xB3,
        ];

        let decoded = [
            995, 989, 173, 996, 173, 173, 173, 992, 992, 992, 992, 992, 992, 992, 992, 992,
        ];

        let (Packet::RcChannelsPacked(channels), _) = Packet::decode(&raw).unwrap() else {
            panic!()
        };

        assert_eq!(decoded, channels.unpack());
    }

    #[test]
    fn encode_rc_channels_packed() {
        let raw: &[u8] = &[
            0xC8, 0x18, 0x16, 0xE3, 0xEB, 0x5E, 0x2B, 0xC8, 0xD7, 0x8A, 0x56, 0xB4, 0x02, 0x7C,
            0xE0, 0x03, 0x1F, 0xF8, 0xC0, 0x07, 0x3E, 0xF0, 0x81, 0x0F, 0x7C, 0xB3,
        ];

        let decoded = [
            995, 989, 173, 996, 173, 173, 173, 992, 992, 992, 992, 992, 992, 992, 992, 992,
        ];

        let (Packet::RcChannelsPacked(channels), _) = Packet::decode(&raw).unwrap() else {
            panic!()
        };

        assert_eq!(decoded, channels.unpack());
    }

    test_pair! {
        enc: encode_link_statistics,
        dec: decode_link_statistics,
        packet: Packet::LinkStatistics {
            up_rssi1: 36,
            up_rssi2: 0,
            up_lq: 100,
            up_snr: 10,
            active_antenna: 0,
            mode: 2,
            tx_power: TxPower::mW_0,
            down_rssi: 0,
            down_lq: 0,
            down_snr: 0,
        },
        raw: &[0xC8, 0x0C, 0x14, 0x24, 0x00, 0x64, 0x0A, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x39],
    }

    test_pair! {
        enc: encode_flight_mode,
        dec: decode_flight_mode,
        packet: Packet::FlightMode(CString::new("Lithobraking!").unwrap()),
        raw: &[0xC8, 0x10, 0x21, 0x4C, 0x69, 0x74, 0x68, 0x6F, 0x62, 0x72, 0x61, 0x6B, 0x69, 0x6E, 0x67, 0x21, 0x00, 0x46],
    }

    test_pair! {
        enc: encode_device_ping,
        dec: decode_device_ping,
        packet: Packet::DevicePing {
            to: Address::Broadcast,
            from: Address::Handset,
        },
        raw: &[0xC8, 0x04, 0x28, 0x00, 0xEA, 0x54],
    }

    test_pair! {
        enc: encode_device_info,
        dec: decode_device_info,
        packet: Packet::DeviceInfo {
            to: Address::Handset,
            from: Address::Transmitter,
            name: CString::new("TEST").unwrap(),
            serial: u32::MAX,
            hardware_version: 1,
            software_version: 2,
            config_parameters: 19,
            config_protocol: 0,
        },
        raw: &[0xC8, 0x17, 0x29, 0xEA, 0xEE, 0x54, 0x45, 0x53, 0x54, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x13, 0x00, 0x8B],
    }
}
