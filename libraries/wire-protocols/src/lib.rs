#![no_std]
#![forbid(unsafe_code)]
#![deny(warnings, clippy::all)]

use bitfield::bitfield;
use core::fmt;

pub mod broadcast;

// TODO - add error variants
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "wire-protocols::Error")
    }
}

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ProtocolIdentifier {
    /// Broadcast protocol ("BRDC")
    Broadcast,
    /// Unknown
    Unknown(u32),
}

impl fmt::Display for ProtocolIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolIdentifier::Broadcast => "broadcast".fmt(f),
            ProtocolIdentifier::Unknown(p) => p.fmt(f),
        }
    }
}

impl From<u32> for ProtocolIdentifier {
    fn from(value: u32) -> Self {
        use ProtocolIdentifier::*;
        match value {
            0x43_44_52_42 => Broadcast,
            _ => Unknown(value),
        }
    }
}

impl From<ProtocolIdentifier> for u32 {
    fn from(value: ProtocolIdentifier) -> Self {
        use ProtocolIdentifier::*;
        match value {
            Broadcast => 0x43_44_52_42,
            Unknown(id) => id,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ProtocolVersion(pub u8);

impl ProtocolVersion {
    pub const fn v1() -> Self {
        ProtocolVersion(1)
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        ProtocolVersion::v1()
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct FirmwareVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl FirmwareVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        FirmwareVersion {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct DeviceId(pub u16);

impl DeviceId {
    pub const fn new(id: u16) -> Self {
        DeviceId(id)
    }

    pub fn as_u16(self) -> u16 {
        self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::UpperHex for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

/// 96 bit device unique serial number identifier
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct DeviceSerialNumber {
    pub word0: u32,
    pub word1: u32,
    pub word2: u32,
}

impl DeviceSerialNumber {
    pub const fn zero() -> Self {
        Self::new(0, 0, 0)
    }

    pub const fn new(word0: u32, word1: u32, word2: u32) -> Self {
        DeviceSerialNumber {
            word0,
            word1,
            word2,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.word0 == 0 && self.word1 == 0 && self.word2 == 0
    }

    pub fn to_le_bytes(self) -> [u8; 12] {
        let id1: [u8; 4] = self.word0.to_le_bytes();
        let id2: [u8; 4] = self.word1.to_le_bytes();
        let id3: [u8; 4] = self.word2.to_le_bytes();
        [
            id3[3], id3[2], id3[1], id3[0], id2[3], id2[2], id2[1], id2[0], id1[3], id1[2], id1[1],
            id1[0],
        ]
    }
}

impl fmt::Display for DeviceSerialNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(self, f)
    }
}

impl fmt::UpperHex for DeviceSerialNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.to_le_bytes().into_iter() {
            fmt::UpperHex::fmt(&c, f)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateTime {
    pub const fn zero() -> Self {
        DateTime {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{}-{} {}:{}:{}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

bitfield! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
    /// TODO docs
    /// Message status flags.
    pub struct StatusFlags(u16);
    impl Debug;
    pub initialized, set_initialized: 1;
    pub datetime_valid, set_datetime_valid: 4;
    pub temperature_valid, set_temperature_valid: 5;
    pub humidity_valid, set_humidity_valid: 6;
    pub voc_ticks_valid, set_voc_ticks_valid: 7;
    pub nox_ticks_valid, set_nox_ticks_valid: 8;
    pub voc_index_valid, set_voc_index_valid: 9;
    pub nox_index_valid, set_nox_index_valid: 10;
    pub pm2_5_valid, set_pm2_5_valid: 11;
    pub co2_valid, set_co2_valid: 12;
}

impl StatusFlags {
    pub const fn empty() -> Self {
        StatusFlags(0)
    }
}

mod field {
    use core::ops;
    pub type Field = ops::Range<usize>;
    pub type Rest = ops::RangeFrom<usize>;
}
