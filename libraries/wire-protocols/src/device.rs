//! The device protocol covers communication related to device information and
//! control and firmware updates, usually over TCP.
//! Everything is little endian.

use byteorder::{ByteOrder, LittleEndian};
use core::fmt;

pub const DEFAULT_PORT: u16 = 32101;
pub const SOCKET_BUFFER_LEN: usize = MemoryRegion::MAX_CHUCK_SIZE + 256;

/// Commands are received by the device.
/// The device always responds with a `StatusCode`, possibly
/// followed by a response type.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Command {
    /// Request device information.
    /// This command also causes the device to reset its connection after sending a response.
    /// It can be used to abort an in-progress update too.
    /// Request type: None
    /// Response type: json string
    Info,

    /// Read a region of FLASH memory.
    /// Request type: MemoryReadRequest
    /// Response type: [u8]
    ReadMemory,

    /// Write a region of FLASH memory.
    /// Request type: MemoryWriteRequest followed by [u8] data
    /// Response type: None
    WriteMemory,

    /// Erase a region of FLASH memory.
    /// Address and length must match one of the boot slots (entire region/all sectors).
    /// Request type: MemoryEraseRequest
    /// Response type: None
    EraseMemory,

    /// Mark the update as complete and schedule a system reboot.
    /// If there was no update in-progress, then this simply reboots the device.
    /// Request type: None
    /// Response type: None
    CompleteAndReboot,

    /// Unknown command.
    /// The device will always response with StatusCode::UnknownCommand.
    /// Request type: None
    /// Response type: None
    Unknown(u32),
}

impl Command {
    pub const WIRE_SIZE: usize = 4;

    pub fn from_le_bytes_unchecked(value: &[u8]) -> Self {
        Command::from(LittleEndian::read_u32(value))
    }

    pub fn from_le_bytes(value: &[u8]) -> Option<Self> {
        if value.len() >= 4 {
            Some(Self::from_le_bytes_unchecked(value))
        } else {
            None
        }
    }
}

impl From<u32> for Command {
    fn from(value: u32) -> Self {
        use Command::*;
        match value {
            1 => Info,
            2 => ReadMemory,
            3 => WriteMemory,
            4 => EraseMemory,
            5 => CompleteAndReboot,
            _ => Unknown(value),
        }
    }
}

impl From<Command> for u32 {
    fn from(value: Command) -> Self {
        use Command::*;
        match value {
            Info => 1,
            ReadMemory => 2,
            WriteMemory => 3,
            EraseMemory => 4,
            CompleteAndReboot => 5,
            Unknown(v) => v,
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MemoryRegion {
    /// Address of the memory region
    pub address: u32,

    /// Size in bytes of the region
    pub length: u32,
}

impl MemoryRegion {
    pub const WIRE_SIZE: usize = 8;
    pub const MAX_CHUCK_SIZE: usize = 1024;

    pub fn new_unchecked(address: u32, length: u32) -> Self {
        Self { address, length }
    }

    pub fn check_length(&self) -> Result<(), StatusCode> {
        if self.length % 4 != 0 {
            Err(StatusCode::LengthNotMultiple4)
        } else if self.length > Self::MAX_CHUCK_SIZE as u32 {
            Err(StatusCode::LengthTooLong)
        } else if self.length == 0 {
            Err(StatusCode::DataLengthIncorrect)
        } else {
            Ok(())
        }
    }

    pub fn to_le_bytes(self) -> [u8; 8] {
        let addr = self.address.to_le_bytes();
        let len = self.length.to_le_bytes();
        [
            addr[0], addr[1], addr[2], addr[3], len[0], len[1], len[2], len[3],
        ]
    }
}

pub type MemoryReadRequest = MemoryRegion;
pub type MemoryWriteRequest = MemoryRegion;
pub type MemoryEraseRequest = MemoryRegion;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum StatusCode {
    Success,
    UnknownCommand,
    InvalidAddress,
    LengthNotMultiple4,
    LengthTooLong,
    DataLengthIncorrect,
    EraseError,
    WriteError,
    FlashError,
    NetworkError,
    InternalError,
    CommandLengthIncorrect,
    Unknown(u32),
}

impl StatusCode {
    pub fn from_le_bytes_unchecked(value: &[u8]) -> Self {
        StatusCode::from(LittleEndian::read_u32(value))
    }

    pub fn from_le_bytes(value: &[u8]) -> Option<Self> {
        if value.len() >= 4 {
            Some(Self::from_le_bytes_unchecked(value))
        } else {
            None
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, StatusCode::Success)
    }
}

impl From<u32> for StatusCode {
    fn from(value: u32) -> Self {
        use StatusCode::*;
        match value {
            0 => Success,
            1 => UnknownCommand,
            2 => InvalidAddress,
            3 => LengthNotMultiple4,
            4 => LengthTooLong,
            5 => DataLengthIncorrect,
            6 => EraseError,
            7 => WriteError,
            8 => FlashError,
            9 => NetworkError,
            10 => InternalError,
            11 => CommandLengthIncorrect,
            _ => Unknown(value),
        }
    }
}

impl From<StatusCode> for u32 {
    fn from(value: StatusCode) -> Self {
        use StatusCode::*;
        match value {
            Success => 0,
            UnknownCommand => 1,
            InvalidAddress => 2,
            LengthNotMultiple4 => 3,
            LengthTooLong => 4,
            DataLengthIncorrect => 5,
            EraseError => 6,
            WriteError => 7,
            FlashError => 8,
            NetworkError => 9,
            InternalError => 10,
            CommandLengthIncorrect => 11,
            Unknown(v) => v,
        }
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_wire_command() {
        for in_c in 0..0xFF_u32 {
            let in_c_bytes = in_c.to_le_bytes();
            let c = Command::from_le_bytes(&in_c_bytes).unwrap();
            assert_eq!(in_c, u32::from(c));
        }
    }

    #[test]
    fn round_trip_status_code() {
        for in_c in 0..0xFF_u32 {
            let in_c_bytes = in_c.to_le_bytes();
            let c = StatusCode::from_le_bytes(&in_c_bytes).unwrap();
            assert_eq!(in_c, u32::from(c));
        }
    }
}
