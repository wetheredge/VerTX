use alloc::vec::Vec;

pub const SECTOR_BYTES: u32 = esp_storage::FlashStorage::SECTOR_SIZE;
const SECTOR_WORDS: usize = SECTOR_BYTES as usize / 4;
const PARTITION_TABLE_ADDRESS: u32 = 0x8000;
const PARTITION_TABLE_SIZE: usize = 0xC00;

const CUSTOM_TYPE_CONFIG: PartitionKind = if let Some(config) = PartitionKind::new_custom(0x40, 0) {
    config
} else {
    // Needed since .unwrap() is not const yet
    panic!("Invalid config PartitionKind")
};

#[derive(Debug)]
pub enum PartitionError {
    UndersizedOtaData,
}

pub fn unlock() -> Result<(), i32> {
    // TODO: What exactly does this do?
    unsafe { esp_storage::ll::spiflash_unlock() }
}

#[allow(clippy::assertions_on_constants)]
pub fn read_partition_table() -> Vec<Result<Partition, PartitionError>> {
    let mut table = [0_u32; PARTITION_TABLE_SIZE / 4];
    const _: () = assert!(PARTITION_TABLE_ADDRESS % SECTOR_BYTES == 0);
    const _: () = assert!(PARTITION_TABLE_SIZE <= SECTOR_BYTES as usize);

    // SAFETY:
    // - Cannot overflow as long as `PARTITION_TABLE_ADDRESS` and `…_SIZE` are
    //   correct. The asserts guarantee it is sector aligned and fits in one sector.
    // - `table` is guaranteed to be word aligned since it is a word array
    unsafe {
        esp_storage::ll::spiflash_read(
            PARTITION_TABLE_ADDRESS,
            table.as_mut_ptr(),
            table.len() as u32 * 4,
        )
    }
    .unwrap();

    let table: &[u8; PARTITION_TABLE_SIZE] = bytemuck::cast_ref(&table);

    table
        .chunks_exact(32)
        .filter(|&chunk| chunk.starts_with(&[0xAA, 0x50]))
        .map_while(|chunk| {
            chunk
                .iter()
                .any(|&b| b != 0xFF)
                .then(|| Partition::parse(chunk))
        })
        .collect()
}

#[derive(Debug)]
pub struct Partition {
    pub name: heapless::Vec<u8, 16>,
    pub kind: PartitionKind,
    pub start: u32,
    pub size: u32,
    pub encrypted: bool,
    pub read_only: bool,
}

impl Partition {
    fn parse(raw: &[u8]) -> Result<Self, PartitionError> {
        let type_ = {
            let type_ = raw[2];
            let sub_type = raw[3];
            let invalid = PartitionKind::Invalid(type_, sub_type);
            match type_ {
                0x00 => AppPartitionKind::from_byte(sub_type).map_or(invalid, PartitionKind::App),
                0x01 => DataPartitionKind::from_byte(sub_type).map_or(invalid, PartitionKind::Data),
                PartitionKind::CUSTOM_MIN..=PartitionKind::CUSTOM_MAX => {
                    PartitionKind::Custom(type_, sub_type)
                }
                _ => invalid,
            }
        };

        let offset = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
        let size = u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]);

        match type_ {
            PartitionKind::Data(DataPartitionKind::Ota) if size < 0x2000 => {
                return Err(PartitionError::UndersizedOtaData);
            }
            _ => {}
        }

        let name = &raw[12..28];
        let name_end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
        let name = &name[0..name_end];
        let name = heapless::Vec::from_slice(name).unwrap();

        let flags = u32::from_le_bytes([raw[28], raw[29], raw[30], raw[31]]);

        Ok(Self {
            name,
            kind: type_,
            start: offset,
            size,
            encrypted: (flags & 1) != 0,
            read_only: (flags & 2) != 0,
        })
    }

    const fn words(&self) -> u32 {
        self.size / 4
    }

    pub const fn sectors(&self) -> u32 {
        self.size / SECTOR_BYTES
    }

    pub const fn is_config(&self) -> bool {
        matches!(self.kind, CUSTOM_TYPE_CONFIG)
    }

    pub const fn is_ota(&self) -> bool {
        match self.kind {
            PartitionKind::App(partition) => partition.is_ota(),
            _ => false,
        }
    }

    pub fn read_into(&self, offset: u32, buffer: &mut [u32]) -> Result<(), i32> {
        self.bounds_check(offset, buffer.len());
        let start = self.start + offset * 4;

        // SAFETY:
        // - `bounds_check` prevents overflowing flash
        // - `&[u32]` ensures correct alignment
        unsafe {
            esp_storage::ll::spiflash_read(start, buffer.as_mut_ptr(), buffer.len() as u32 * 4)
        }
    }

    pub fn erase_sector(&mut self, sector: u32) -> Result<(), i32> {
        assert!(sector < self.sectors());
        // SAFETY: `assert!` prevents overflowing flash
        unsafe { esp_storage::ll::spiflash_erase_sector(sector) }
    }

    pub fn write(&mut self, offset: u32, data: &[u32]) -> Result<(), i32> {
        self.bounds_check(offset, data.len());
        let start = self.start + offset * 4;

        // SAFETY:
        // - `bounds_check` prevents overflowing flash
        // - `&[u32]` ensures correct alignment
        unsafe { esp_storage::ll::spiflash_write(start, data.as_ptr(), data.len() as u32 * 4) }
    }

    pub fn erase_and_write(&mut self, offset: u32, data: &[u32]) -> Result<(), i32> {
        self.bounds_check(offset, data.len());
        let start = self.start + offset * 4;

        let first_sector = start / SECTOR_BYTES;
        let sector_count = (data.len() as u32).div_ceil(SECTOR_BYTES);
        for sector in (0..sector_count).map(|x| x + first_sector) {
            // SAFETY: `bounds_check` prevents overflowing flash
            unsafe { esp_storage::ll::spiflash_erase_sector(sector) }?;
        }

        // SAFETY:
        // - `bounds_check` prevents overflowing flash
        // - `&[u32]` ensures correct alignment
        unsafe { esp_storage::ll::spiflash_write(start, data.as_ptr(), data.len() as u32 * 4) }
    }

    fn bounds_check(&self, offset: u32, length: usize) {
        assert!((length as u32 + offset) <= self.words());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionKind {
    App(AppPartitionKind),
    Data(DataPartitionKind),
    Custom(u8, u8),
    Invalid(u8, u8),
}

impl PartitionKind {
    const CUSTOM_MAX: u8 = 0xFE;
    const CUSTOM_MIN: u8 = 0x40;

    pub const fn new_custom(type_: u8, sub_type: u8) -> Option<Self> {
        match type_ {
            PartitionKind::CUSTOM_MIN..=PartitionKind::CUSTOM_MAX => {
                Some(Self::Custom(type_, sub_type))
            }
            _ => None,
        }
    }
}

macro_rules! byte_enum {
    ($(#[$attr:meta])* $pub:vis enum $name:ident { $( $variant:ident = $value:literal ),* $(,)? }) => {
        $(#[$attr])*
        #[repr(u8)]
        $pub enum $name {
            $( $variant = $value ),*
        }

        impl $name {
            fn from_byte(byte: u8) -> Option<Self> {
                match byte {
                    $( $value => Some(Self::$variant), )*
                    _ => None,
                }
            }
        }
    }
}

byte_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum AppPartitionKind {
        Factory = 0x00,
        Ota0 = 0x10,
        Ota1 = 0x11,
        Ota2 = 0x12,
        Ota3 = 0x13,
        Ota4 = 0x14,
        Ota5 = 0x15,
        Ota6 = 0x16,
        Ota7 = 0x17,
        Ota8 = 0x18,
        Ota9 = 0x19,
        Ota10 = 0x1A,
        Ota11 = 0x1B,
        Ota12 = 0x1C,
        Ota13 = 0x1D,
        Ota14 = 0x1E,
        Ota15 = 0x1F,
        Test = 0x20,
    }
}

impl AppPartitionKind {
    const fn is_ota(self) -> bool {
        (self as u8) & 0xF0 == 0x10
    }
}

byte_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum DataPartitionKind {
        Ota = 0x00,
        Phy = 0x01,
        Nvs = 0x02,
        Coredump = 0x03,
        NvsKeys = 0x04,
        EFuse = 0x05,
        Undefined = 0x06,
        Fat = 0x81,
        Spiffs = 0x82,
        LittleFs = 0x83,
    }
}
