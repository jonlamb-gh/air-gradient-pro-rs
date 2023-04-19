use bootloader_support::{BootSlot, FLASH_BASE_ADDRESS};
use core::{mem, ptr};
use log::debug;
use static_assertions::const_assert_eq;
use stm32f4xx_hal::{crc32::Crc32, flash::FlashExt};

const_assert_eq!(mem::align_of::<BootConfig>(), 4);
const_assert_eq!(mem::size_of::<BootConfig>(), BootConfig::SIZE_IN_FLASH);

pub static DEFAULT_CONFIG: BootConfig = BootConfig {
    magic: 0,
    version: 0,
    firmware_boot_slot: BootSlot::Slot0,
    checksum: 0,
};

/// Boot configuration.
/// Lives in flash sector 3 (0x0800_C000).
/// `magic` is set to `BootConfig::MAGIC`
/// `checksum` is the CRC32 of the preceeding bytes.
#[derive(Copy, Clone)]
pub struct BootConfig {
    magic: u32,
    version: u32,
    firmware_boot_slot: BootSlot,
    checksum: u32,
}

impl BootConfig {
    /// The flash address of the boot configuration.
    const FLASH_ADDRESS: u32 = FLASH_BASE_ADDRESS + Self::FLASH_SECTOR_OFFSET;
    const FLASH_SECTOR: usize = 3;
    const FLASH_SECTOR_OFFSET: u32 = 0xC000;

    const SIZE_IN_FLASH: usize = 16;

    const MAGIC: u32 = 0xFEEDC0DE;

    pub fn read<F: FlashExt>(flash: &F, crc: &mut Crc32) -> Option<Self> {
        debug!(
            "Reading boot config from flash 0x{:X} (offset 0x{:X})",
            Self::FLASH_ADDRESS,
            Self::FLASH_SECTOR_OFFSET
        );

        let cfg_bytes = &flash.read()[Self::FLASH_SECTOR_OFFSET as usize..];
        let cfg = BootConfig {
            magic: u32::from_le_bytes(cfg_bytes[0..4].try_into().unwrap()),
            version: u32::from_le_bytes(cfg_bytes[4..8].try_into().unwrap()),
            firmware_boot_slot: BootSlot::from_u32(u32::from_le_bytes(
                cfg_bytes[8..12].try_into().unwrap(),
            )),
            checksum: u32::from_le_bytes(cfg_bytes[12..16].try_into().unwrap()),
        };

        if cfg.magic != Self::MAGIC {
            debug!("Config has invalid magic 0x{:X}", cfg.magic);
            None
        } else {
            crc.init();
            // -4 to exclude the checksum field
            let expected_crc = crc.update_bytes(&cfg_bytes[..Self::SIZE_IN_FLASH - 4]);
            if cfg.checksum != expected_crc {
                debug!(
                    "Config has invalid checksum 0x{:X} (expected 0x{expected_crc:X})",
                    cfg.checksum
                );
                None
            } else {
                Some(cfg)
            }
        }
    }

    pub fn write<F: FlashExt>(&mut self, flash: &mut F, crc: &mut Crc32) {
        crc.init();
        self.magic = Self::MAGIC;
        crc.update(&[self.magic]);
        crc.update(&[self.version]);
        let crc = crc.update(&[self.firmware_boot_slot.into_u32()]);
        self.checksum = crc;

        let bytes = self.convert_to_le_bytes();
        let mut unlocked_flash = flash.unlocked();
        unlocked_flash.erase(Self::FLASH_SECTOR as u8).unwrap();
        unlocked_flash
            .program(Self::FLASH_SECTOR_OFFSET as usize, bytes.iter())
            .unwrap()
    }

    pub fn firmware_boot_slot(&self) -> BootSlot {
        self.firmware_boot_slot
    }

    pub fn swap_firmware_boot_slot(&mut self) {
        self.firmware_boot_slot = self.firmware_boot_slot.other();
    }

    fn convert_to_le_bytes(&self) -> [u8; Self::SIZE_IN_FLASH] {
        let a = self.magic.to_le_bytes();
        let b = self.version.to_le_bytes();
        let c = self.firmware_boot_slot.into_u32().to_le_bytes();
        let d = self.checksum.to_le_bytes();
        [
            a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3], d[0], d[1],
            d[2], d[3],
        ]
    }
}

pub trait BootSlotExt {
    fn application_flash_address(&self) -> Option<u32>;

    fn into_u32(self) -> u32;

    fn from_u32(value: u32) -> Self;
}

impl BootSlotExt for BootSlot {
    fn application_flash_address(&self) -> Option<u32> {
        let addr = self.address();
        let sp_ptr = addr as *const u32;
        let sp = unsafe { ptr::read_volatile(sp_ptr) };
        // TODO - check if in RAM
        let reset_vector_ptr = unsafe { sp_ptr.offset(1) };
        let reset_vector = unsafe { ptr::read_volatile(reset_vector_ptr) };
        debug!("addr = 0x{addr:X} sp = 0x{sp:X} rv = 0x{reset_vector:X}");
        if self.contains(reset_vector) {
            Some(addr)
        } else {
            None
        }
    }

    fn into_u32(self) -> u32 {
        use BootSlot::*;
        match self {
            Slot0 => 0,
            Slot1 => 1,
        }
    }

    fn from_u32(value: u32) -> Self {
        if value == 0 {
            BootSlot::Slot0
        } else {
            BootSlot::Slot1
        }
    }
}
