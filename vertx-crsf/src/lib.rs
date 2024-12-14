#![no_std]

extern crate alloc;

use alloc::ffi::CString;
use alloc::vec;
use core::fmt;

use crc::Crc;

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

#[expect(unused)]
const EXTENDED_CRC: Crc<u8> = Crc::<u8>::new(&crc::Algorithm {
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
    RcChannelsPacked(RcChannelsPacked<[u8; 22]>),
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
    DeviceInfo,
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
    #[expect(non_camel_case_types)]
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

bitfield::bitfield! {
    #[derive(Debug, Clone, PartialEq)]
    pub struct RcChannelsPacked([u8]);
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

impl RcChannelsPacked<[u8; 22]> {
    pub fn unpack(&self) -> [u16; 16] {
        [
            self.channel0(),
            self.channel1(),
            self.channel2(),
            self.channel3(),
            self.channel4(),
            self.channel5(),
            self.channel6(),
            self.channel7(),
            self.channel8(),
            self.channel9(),
            self.channel10(),
            self.channel11(),
            self.channel12(),
            self.channel13(),
            self.channel14(),
            self.channel15(),
        ]
    }
}

#[derive(Debug)]
pub enum PacketError<E> {
    ReadError(E),
    UnexpectedEof,
    InvalidSyncByte(u8),
    InvalidPacketKind(u8),
    BadCrc,
}

impl<E> From<embedded_io::ReadExactError<E>> for PacketError<E> {
    fn from(err: embedded_io::ReadExactError<E>) -> Self {
        match err {
            embedded_io::ReadExactError::UnexpectedEof => Self::UnexpectedEof,
            embedded_io::ReadExactError::Other(err) => Self::ReadError(err),
        }
    }
}

impl Packet {
    pub fn read<R: embedded_io::Read<Error = E>, E>(raw: &mut R) -> Result<Self, PacketError<E>> {
        let mut reader = PacketReader::new(raw)?;

        let packet = if reader.packet_type < 0x28 {
            match reader.packet_type {
                // CRSF_FRAMETYPE_GPS
                0x02 => Self::Gps {
                    latitude: reader.i32(),
                    longitude: reader.i32(),
                    speed: reader.i16(),
                    heading: reader.i16(),
                    altitude: i32::from(reader.u16()) - 1000,
                    satellites: reader.u8(),
                },
                // CRSF_FRAMETYPE_VARIO
                0x07 => Self::Vario {
                    vertical_speed: reader.i16(),
                },
                // CRSF_FRAMETYPE_BATTERY_SENSOR
                0x08 => Self::BatterySensor {
                    voltage: reader.i16(),
                    current: reader.i16(),
                    used: reader.i24(),
                    remaining: reader.i8(),
                },
                // CRSF_FRAMETYPE_BARO_ALTITUDE
                0x09 => Self::BarometricAltitude {
                    altitude: convert_barometric_altitude(reader.i16()),
                    vertical_speed: (reader.payload_length == 2).then(|| reader.i16()),
                },
                // CRSF_FRAMETYPE_HEARTBEAT
                0x0B => {
                    // FIXME
                    let origin = Address::from_u16(reader.u16()).unwrap();

                    Self::Heartbeat { origin }
                }
                // CRSF_FRAMETYPE_LINK_STATISTICS
                0x14 => {
                    Self::LinkStatistics {
                        up_rssi1: reader.u8(),
                        up_rssi2: reader.u8(),
                        up_lq: reader.u8(),
                        up_snr: reader.i8(),
                        active_antenna: reader.u8(),
                        mode: reader.u8(),
                        // FIXME
                        tx_power: TxPower::from_raw(reader.u8()).unwrap(),
                        down_rssi: reader.u8(),
                        down_lq: reader.u8(),
                        down_snr: reader.i8(),
                    }
                }
                // CRSF_FRAMETYPE_RC_CHANNELS_PACKED
                0x16 => {
                    // FIXME
                    let mut channels = [0; 22];
                    channels.clone_from_slice(reader.payload());
                    Self::RcChannelsPacked(RcChannelsPacked(channels))
                }
                // CRSF_FRAMETYPE_SUBSET_RC_CHANNELS_PACKED
                0x17 => todo!(),
                // CRSF_FRAMETYPE_LINK_RX_ID
                0x1C => todo!(),
                // CRSF_FRAMETYPE_LINK_TX_ID
                0x1D => todo!(),
                // CRSF_FRAMETYPE_ATTITUDE
                0x1E => todo!(),
                // CRSF_FRAMETYPE_FLIGHT_MODE
                0x21 => {
                    let length = reader.payload_length.into();
                    // FIXME
                    let mut data = vec![0; length];
                    data.clone_from_slice(reader.payload());
                    // Strip trailing 0
                    data.truncate(length - 1);

                    // FIXME: return error instead of unwrapping?
                    let mode = CString::new(data).unwrap();
                    Self::FlightMode(mode)
                }

                kind => return Err(PacketError::InvalidPacketKind(kind)),
            }
        } else {
            let reader = reader.extended()?;

            match reader.packet_type() {
                // CRSF_FRAMETYPE_DEVICE_PING
                0x28 => Self::DevicePing {
                    to: reader.to,
                    from: reader.from,
                },
                // CRSF_FRAMETYPE_DEVICE_INFO
                0x29 => todo!(), // extended
                // CRSF_FRAMETYPE_PARAMETER_SETTINGS_ENTRY
                0x2B => todo!(), // extended
                // CRSF_FRAMETYPE_PARAMETER_READ
                0x2C => todo!(), // extended
                // CRSF_FRAMETYPE_PARAMETER_WRITE
                0x2D => todo!(), // extended
                // CRSF_FRAMETYPE_ELRS_STATUS
                0x2E => todo!(), // extended
                // CRSF_FRAMETYPE_COMMAND
                0x32 => todo!(), // extended
                // CRSF_FRAMETYPE_RADIO_ID
                0x3A => todo!(), // extended
                // CRSF_FRAMETYPE_KISS_REQ
                0x78 => todo!(), // extended
                // CRSF_FRAMETYPE_KISS_RESP
                0x79 => todo!(), // extended
                // CRSF_FRAMETYPE_MSP_REQ
                0x7A => todo!(), // extended
                // CRSF_FRAMETYPE_MSP_RESP
                0x7B => todo!(), // extended
                // CRSF_FRAMETYPE_MSP_WRITE
                0x7C => todo!(), // extended
                // CRSF_FRAMETYPE_DISPLAYPORT_CMD
                0x7D => todo!(), // extended
                // CRSF_FRAMETYPE_ARDUPILOT_RESP
                0x80 => todo!(), // extended

                kind => return Err(PacketError::InvalidPacketKind(kind)),
            }
        };

        Ok(packet)
    }
}

fn convert_barometric_altitude(altitude: i16) -> i32 {
    if altitude.is_negative() {
        i32::from(altitude & 0x7FFF) * 10
    } else {
        i32::from(altitude) - 10_000
    }
}

#[derive(Debug)]
struct PacketReader {
    /// Max packet size excluding sync, length, and type bytes
    payload: [u8; 61],
    /// Next byte index to read from
    next: usize,
    /// Actual packet size excluding sync, length, type, and crc bytes
    payload_length: u8,
    packet_type: u8,
}

impl PacketReader {
    fn new<R, E>(reader: &mut R) -> Result<Self, PacketError<E>>
    where
        R: embedded_io::Read<Error = E>,
    {
        let mut buffer = [0; 3];
        reader.read_exact(&mut buffer)?;
        let [sync, length, packet_type] = buffer;

        if sync != 0xC8 {
            return Err(PacketError::InvalidSyncByte(sync));
        }

        // Exclude type and crc bytes
        let payload_length = length - 2;

        let mut payload = [0; 61];
        reader.read_exact(&mut payload[0..payload_length as usize])?;

        let mut checksum = [0];
        reader.read_exact(&mut checksum)?;
        let [checksum] = checksum;

        let mut crc = BASIC_CRC.digest();
        crc.update(&[packet_type]);
        crc.update(&payload[0..payload_length.into()]);

        if crc.finalize() != checksum {
            return Err(PacketError::BadCrc);
        }

        Ok(Self {
            payload,
            next: 0,
            payload_length,
            packet_type,
        })
    }

    fn extended<E>(&mut self) -> Result<ExtendedPacketReader<'_>, PacketError<E>> {
        ExtendedPacketReader::new(self)
    }

    fn payload(&mut self) -> &[u8] {
        let rest = &self.payload[self.next..self.payload_length.into()];
        self.next = self.payload_length.into();
        rest
    }

    fn u8(&mut self) -> u8 {
        let x = u8::from_be(self.payload[self.next]);
        self.next += 1;
        x
    }

    fn i8(&mut self) -> i8 {
        self.u8() as i8
    }

    fn u16(&mut self) -> u16 {
        let bytes = [self.payload[self.next], self.payload[self.next + 1]];
        self.next += 2;
        u16::from_be_bytes(bytes)
    }

    fn i16(&mut self) -> i16 {
        self.u16() as i16
    }

    fn i24(&mut self) -> i32 {
        let b1 = self.payload[self.next];
        let b2 = self.payload[self.next + 1];
        let b3 = self.payload[self.next + 2];
        let b4 = if b3.leading_ones() > 0 { 0xFF } else { 0x00 };
        self.next += 3;

        i32::from_be_bytes([b1, b2, b3, b4])
    }

    fn u32(&mut self) -> u32 {
        let bytes = [
            self.payload[self.next],
            self.payload[self.next + 1],
            self.payload[self.next + 2],
            self.payload[self.next + 3],
        ];
        self.next += 4;
        u32::from_be_bytes(bytes)
    }

    fn i32(&mut self) -> i32 {
        self.u32() as i32
    }
}

impl Drop for PacketReader {
    fn drop(&mut self) {
        if cfg!(debug_assertions) && self.next != self.payload_length.into() {
            panic!(
                "Expected to read {} bytes. Actually read {} bytes",
                self.payload_length, self.next
            );
        }
    }
}

#[derive(Debug)]
struct ExtendedPacketReader<'a> {
    reader: &'a mut PacketReader,
    to: Address,
    from: Address,
}

impl<'a> ExtendedPacketReader<'a> {
    fn new<E>(reader: &'a mut PacketReader) -> Result<Self, PacketError<E>> {
        // FIXME: return error
        let to = Address::from_raw(reader.u8()).unwrap();
        let from = Address::from_raw(reader.u8()).unwrap();

        Ok(Self { reader, to, from })
    }

    fn packet_type(&self) -> u8 {
        self.reader.packet_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_rc_channels_packed_all_1500() {
        let mut raw: &[u8] = &[
            0xC8, 0x18, 0x16, 0xE0, 0x03, 0x1F, 0xF8, 0xC0, 0x07, 0x3E, 0xF0, 0x81, 0x0F, 0x7C,
            0xE0, 0x03, 0x1F, 0xF8, 0xC0, 0x07, 0x3E, 0xF0, 0x81, 0x0F, 0x7C, 0xAD,
        ];

        let Packet::RcChannelsPacked(channels) = Packet::read(&mut raw).unwrap() else {
            panic!()
        };

        assert!(channels.unpack().into_iter().all(|ch| ch == 992));
    }

    #[test]
    fn packet_link_statistics() {
        let mut raw: &[u8] = &[
            0xC8, 0x0C, 0x14, 0x24, 0x00, 0x64, 0x0A, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x39,
            // 0xC8, 0x0C, 0x14, 0x6C, 0x00, 0x00, 0x0B, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x88,
        ];

        let expected = Packet::LinkStatistics {
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
        };

        assert_eq!(expected, Packet::read(&mut raw).unwrap());
    }

    #[test]
    fn packet_flight_mode() {
        let mut raw: &[u8] = &[
            0xC8, 0x10, 0x21, 0x4C, 0x69, 0x74, 0x68, 0x6F, 0x62, 0x72, 0x61, 0x6B, 0x69, 0x6E,
            0x67, 0x21, 0x00, 0x46,
        ];

        let expected = Packet::FlightMode(CString::new("Lithobraking!").unwrap());

        assert_eq!(expected, Packet::read(&mut raw).unwrap());
    }
}
