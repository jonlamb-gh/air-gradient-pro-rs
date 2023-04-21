#![no_std]
#![forbid(unsafe_code)]
#![deny(warnings, clippy::all)]

use core::{fmt, str};

mod reset_reason;

pub use self::reset_reason::ResetReason;

pub const FLASH_BASE_ADDRESS: u32 = 0x0800_0000;

pub const FLASH_SLOT0_ADDRESS: u32 = FLASH_BASE_ADDRESS + FLASH_SLOT0_SECTOR_OFFSET;
pub const FLASH_SLOT1_ADDRESS: u32 = FLASH_BASE_ADDRESS + FLASH_SLOT1_SECTOR_OFFSET;

/// Sector 4
pub const FLASH_SLOT0_SECTOR_OFFSET: u32 = 0x1_0000;

/// Sector 6
pub const FLASH_SLOT1_SECTOR_OFFSET: u32 = 0x4_0000;

/// Slot size is 194K bytes
pub const FLASH_SLOT_SIZE: u32 = 194 * 1024;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum BootSlot {
    Slot0,
    Slot1,
}

impl BootSlot {
    pub fn other(&self) -> Self {
        match self {
            BootSlot::Slot0 => BootSlot::Slot1,
            BootSlot::Slot1 => BootSlot::Slot0,
        }
    }

    pub fn address(&self) -> u32 {
        match self {
            BootSlot::Slot0 => FLASH_SLOT0_ADDRESS,
            BootSlot::Slot1 => FLASH_SLOT1_ADDRESS,
        }
    }

    pub fn offset(&self) -> u32 {
        match self {
            BootSlot::Slot0 => FLASH_SLOT0_SECTOR_OFFSET,
            BootSlot::Slot1 => FLASH_SLOT1_SECTOR_OFFSET,
        }
    }

    pub fn size(&self) -> u32 {
        FLASH_SLOT_SIZE
    }

    pub fn contains(&self, address: u32) -> bool {
        self.address() <= address && address < (self.address() + self.size())
    }

    pub fn sectors(&self) -> &'static [u8] {
        match self {
            BootSlot::Slot0 => &[4, 5],
            BootSlot::Slot1 => &[6, 7],
        }
    }
}

impl fmt::Display for BootSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BootSlot::*;
        match self {
            Slot0 => f.write_str("SLOT0"),
            Slot1 => f.write_str("SLOT1"),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ParseBootSlotError;

impl fmt::Display for ParseBootSlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Error parsing boot slot from str")
    }
}

impl str::FromStr for BootSlot {
    type Err = ParseBootSlotError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim() {
            "SLOT0" => BootSlot::Slot0,
            "SLOT1" => BootSlot::Slot1,
            _ => return Err(ParseBootSlotError),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn slot0() {
        assert_eq!(BootSlot::Slot0.offset(), 0x1_0000);
        assert_eq!(BootSlot::Slot0.address(), 0x0801_0000);
        assert_eq!(BootSlot::Slot0.size(), 194 * 1024);
        assert_eq!(BootSlot::Slot0.other(), BootSlot::Slot1);
        assert_eq!(BootSlot::Slot0.contains(0x0801_0000 - 1), false);
        assert_eq!(BootSlot::Slot0.contains(0x0801_0000), true);
        assert_eq!(
            BootSlot::Slot0.contains(0x0801_0000 + (194 * 1024) - 1),
            true
        );
        assert_eq!(BootSlot::Slot0.contains(0x0801_0000 + (194 * 1024)), false);
    }

    #[test]
    fn slot1() {
        assert_eq!(BootSlot::Slot1.offset(), 0x4_0000);
        assert_eq!(BootSlot::Slot1.address(), 0x0804_0000);
        assert_eq!(BootSlot::Slot1.size(), 194 * 1024);
        assert_eq!(BootSlot::Slot1.other(), BootSlot::Slot0);
        assert_eq!(BootSlot::Slot1.contains(0x0804_0000 - 1), false);
        assert_eq!(BootSlot::Slot1.contains(0x0804_0000), true);
        assert_eq!(
            BootSlot::Slot1.contains(0x0804_0000 + (194 * 1024) - 1),
            true
        );
        assert_eq!(BootSlot::Slot1.contains(0x0804_0000 + (194 * 1024)), false);
    }

    #[test]
    fn from_str() {
        assert_eq!(BootSlot::from_str(" SLOT0  "), Ok(BootSlot::Slot0));
        assert_eq!(BootSlot::from_str("   SLOT1  "), Ok(BootSlot::Slot1));
        assert!(BootSlot::from_str("slot2").is_err());
    }
}
