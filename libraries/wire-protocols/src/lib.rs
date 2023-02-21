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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FirmwareVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DeviceId(pub u16);

/// 96 bit device unique serial number identifier
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DeviceSerialNumber {
    pub word0: u32,
    pub word1: u32,
    pub word2: u32,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

bitfield! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    /// TODO docs
    pub struct StatusFlags(u16);
    impl Debug;
    pub foo, set_foo: 0;
}

mod field {
    use core::ops;
    pub type Field = ops::Range<usize>;
    pub type Rest = ops::RangeFrom<usize>;
}
