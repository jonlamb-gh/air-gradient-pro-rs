//! The device protocol covers communication related to device information and
//! control and firmware updates, usually over TCP.
//! Everything is little endian.

/// Commands are received by the device.
/// The device always responds with a `StatusCode`, possibly
/// followed by a response type.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Command {
    /// Request device information.
    /// Request type: None
    /// Response type: json string
    Info,

    /// Read a region of FLASH memory.
    /// Request type: MemoryReadRequest
    /// Response type: [u8]
    ReadMemory,

    /// Erase a region of FLASH memory.
    /// Request type: MemoryEraseRequest
    /// Response type: None
    EraseMemory,

    /// Write a region of FLASH memory.
    /// Request type: MemoryWriteRequest followed by [u8] data
    /// Response type: None
    WriteMemory,

    /// Schedule a system reboot.
    /// Request type: None
    /// Response type: None
    Reboot,

    /// Unknown command
    /// Request type: None
    /// Response type: None
    Unknown(u32),
}

impl From<u32> for Command {
    fn from(value: u32) -> Self {
        use Command::*;
        match value {
            1 => Info,
            2 => ReadMemory,
            3 => EraseMemory,
            4 => WriteMemory,
            5 => Reboot,
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
            EraseMemory => 3,
            WriteMemory => 4,
            Reboot => 5,
            Unknown(v) => v,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MemoryRegion {
    /// Address of the memory region
    pub address: u32,

    /// Size in bytes of the region
    pub length: u32,
}

pub type MemoryReadRequest = MemoryRegion;
pub type MemoryEraseRequest = MemoryRegion;
pub type MemoryWriteRequest = MemoryRegion;

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
    Unknown(u32),
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
            Unknown(v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_wire_command() {
        for in_c in 0..0xFF_u32 {
            let c = Command::from(in_c);
            assert_eq!(in_c, u32::from(c));
        }
    }

    #[test]
    fn round_trip_status_code() {
        for in_c in 0..0xFF_u32 {
            let c = StatusCode::from(in_c);
            assert_eq!(in_c, u32::from(c));
        }
    }
}
