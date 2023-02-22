#![deny(warnings, clippy::all)]
#![forbid(unsafe_code)]
#![no_std]

use bitfield::bitfield;
use core::fmt;

pub mod broadcast;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "wire-protocols::Error")
    }
}

pub type Result<T> = core::result::Result<T, Error>;

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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct DeviceId(pub u16);

impl DeviceId {
    pub const fn new(id: u16) -> Self {
        DeviceId(id)
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
    pub const fn new(word0: u32, word1: u32, word2: u32) -> Self {
        DeviceSerialNumber {
            word0,
            word1,
            word2,
        }
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

bitfield! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
    /// TODO docs
    pub struct StatusFlags(u16);
    impl Debug;
    pub temperature_valid, set_temperature_valid: 8;
    pub humidity_valid, set_humidity_valid: 9;
    pub voc_ticks_valid, set_voc_ticks_valid: 10;
    pub nox_ticks_valid, set_nox_ticks_valid: 11;
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
