//! A small fixed-size message that is broadcast periodically
//! (usually over UDP, on the order of seconds to minutes).

use crate::{
    DateTime, DeviceId, DeviceSerialNumber, Error, FirmwareVersion, ProtocolIdentifier,
    ProtocolVersion, Result, StatusFlags,
};
use byteorder::{ByteOrder, LittleEndian};
use core::fmt;

pub const DEFAULT_PORT: u16 = 32100;

#[derive(Debug, Clone)]
pub struct Message<T: AsRef<[u8]>> {
    buffer: T,
}

mod field {
    use crate::field::*;

    pub const PROTOCOL: Field = 0..4;
    pub const PROTOCOL_VERSION: usize = 4;
    pub const FIRMWARE_VERSION_PATCH: Field = 5..7;
    pub const FIRMWARE_VERSION_MINOR: Field = 7..9;
    pub const FIRMWARE_VERSION_MAJOR: Field = 9..11;

    pub const DEVICE_ID: Field = 11..13;
    pub const DEVICE_SERIAL_NUMBER0: Field = 13..17;
    pub const DEVICE_SERIAL_NUMBER1: Field = 17..21;
    pub const DEVICE_SERIAL_NUMBER2: Field = 21..25;

    pub const SEQUENCE_NUMBER: Field = 25..29;
    pub const UPTIME_SECONDS: Field = 29..33;

    pub const STATUS_FLAGS: Field = 33..35;

    pub const DATETIME_YEAR: Field = 35..37;
    pub const DATETIME_MONTH: usize = 37;
    pub const DATETIME_DAY: usize = 38;
    pub const DATETIME_HOUR: usize = 39;
    pub const DATETIME_MINUTE: usize = 40;
    pub const DATETIME_SECOND: usize = 41;

    pub const TEMPERATURE: Field = 42..46;
    pub const HUMIDITY: Field = 46..48;
    pub const VOC_TICKS: Field = 48..50;
    pub const NOX_TICKS: Field = 50..52;
    pub const VOC_INDEX: Field = 52..54;
    pub const NOX_INDEX: Field = 54..56;
    pub const PM2_5_ATM: Field = 56..58;
    pub const CO2: Field = 58..60;

    pub const REST: Rest = 60..;
}

/// The fixed-size message length.
pub const MESSAGE_LEN: usize = field::REST.start;

impl<T: AsRef<[u8]>> Message<T> {
    /// Imbue a raw octet buffer with message structure.
    pub const fn new_unchecked(buffer: T) -> Message<T> {
        Message { buffer }
    }

    /// Shorthand for a combination of [new_unchecked] and [check_len].
    ///
    /// [new_unchecked]: #method.new_unchecked
    /// [check_len]: #method.check_len
    pub fn new_checked(buffer: T) -> Result<Message<T>> {
        let packet = Self::new_unchecked(buffer);
        packet.check_len()?;
        packet.check_protocol()?;
        Ok(packet)
    }

    /// Ensure that no accessor method will panic if called.
    /// Returns `Err(Error)` if the buffer is too short.
    pub fn check_len(&self) -> Result<()> {
        let len = self.buffer.as_ref().len();
        if len < MESSAGE_LEN {
            Err(Error)
        } else {
            Ok(())
        }
    }

    /// Check that the message protocol matches the
    /// expected broadcast protocol identifier.
    pub fn check_protocol(&self) -> Result<()> {
        if self.protocol() != ProtocolIdentifier::Broadcast {
            Err(Error)
        } else {
            Ok(())
        }
    }

    /// Consumes the message, returning the underlying buffer.
    pub fn into_inner(self) -> T {
        self.buffer
    }

    /// Return the length of a message.
    pub const fn message_len() -> usize {
        MESSAGE_LEN
    }

    /// Return the protocol field.
    #[inline]
    pub fn protocol(&self) -> ProtocolIdentifier {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::PROTOCOL]).into()
    }

    /// Return the protocol version field.
    #[inline]
    pub fn protocol_version(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[field::PROTOCOL_VERSION]
    }

    /// Return the firmware version patch field.
    #[inline]
    pub fn firmware_version_patch(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::FIRMWARE_VERSION_PATCH])
    }

    /// Return the firmware version minor field.
    #[inline]
    pub fn firmware_version_minor(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::FIRMWARE_VERSION_MINOR])
    }

    /// Return the firmware version major field.
    #[inline]
    pub fn firmware_version_major(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::FIRMWARE_VERSION_MAJOR])
    }

    /// Return the device ID field.
    #[inline]
    pub fn device_id(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::DEVICE_ID])
    }

    /// Return the device serial number word 0 field.
    #[inline]
    pub fn device_serial_number_word0(&self) -> u32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::DEVICE_SERIAL_NUMBER0])
    }

    /// Return the device serial number word 1 field.
    #[inline]
    pub fn device_serial_number_word1(&self) -> u32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::DEVICE_SERIAL_NUMBER1])
    }

    /// Return the device serial number word 2 field.
    #[inline]
    pub fn device_serial_number_word2(&self) -> u32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::DEVICE_SERIAL_NUMBER2])
    }

    /// Return the sequence number field.
    #[inline]
    pub fn sequence_number(&self) -> u32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::SEQUENCE_NUMBER])
    }

    /// Return the uptime seconds field.
    #[inline]
    pub fn uptime_seconds(&self) -> u32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::UPTIME_SECONDS])
    }

    /// Return the status flags field.
    #[inline]
    pub fn status_flags(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::STATUS_FLAGS])
    }

    /// Return the date-time year field.
    #[inline]
    pub fn datetime_year(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::DATETIME_YEAR])
    }

    /// Return the date-time month field.
    #[inline]
    pub fn datetime_month(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[field::DATETIME_MONTH]
    }

    /// Return the date-time day field.
    #[inline]
    pub fn datetime_day(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[field::DATETIME_DAY]
    }

    /// Return the date-time hour field.
    #[inline]
    pub fn datetime_hour(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[field::DATETIME_HOUR]
    }

    /// Return the date-time minute field.
    #[inline]
    pub fn datetime_minute(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[field::DATETIME_MINUTE]
    }

    /// Return the date-time second field.
    #[inline]
    pub fn datetime_second(&self) -> u8 {
        let data = self.buffer.as_ref();
        data[field::DATETIME_SECOND]
    }

    /// Return the temperature field.
    #[inline]
    pub fn temperature(&self) -> i32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_i32(&data[field::TEMPERATURE])
    }

    /// Return the humidity field.
    #[inline]
    pub fn humidity(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::HUMIDITY])
    }

    /// Return the VOC ticks field.
    #[inline]
    pub fn voc_ticks(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::VOC_TICKS])
    }

    /// Return the NOx ticks field.
    #[inline]
    pub fn nox_ticks(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::NOX_TICKS])
    }

    /// Return the VOC index field.
    #[inline]
    pub fn voc_index(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::VOC_INDEX])
    }

    /// Return the NOx index field.
    #[inline]
    pub fn nox_index(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::NOX_INDEX])
    }

    /// Return the PM2.5 (under atmospheric environment) field.
    #[inline]
    pub fn pm2_5_atm(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::PM2_5_ATM])
    }

    /// Return the CO2 index field.
    #[inline]
    pub fn co2(&self) -> u16 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u16(&data[field::CO2])
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Message<&'a T> {
    /// Return a pointer to the remaining data following a message, if any.
    #[inline]
    pub fn rest(&self) -> &'a [u8] {
        let data = self.buffer.as_ref();
        &data[field::REST]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Message<T> {
    /// Set the protocol field.
    #[inline]
    pub fn set_protocol(&mut self, value: ProtocolIdentifier) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u32(&mut data[field::PROTOCOL], value.into())
    }

    /// Set the protocol version field.
    #[inline]
    pub fn set_protocol_version(&mut self, value: u8) {
        let data = self.buffer.as_mut();
        data[field::PROTOCOL_VERSION] = value;
    }

    /// Set the firmware version patch field.
    #[inline]
    pub fn set_firmware_version_patch(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::FIRMWARE_VERSION_PATCH], value)
    }

    /// Set the firmware version minor field.
    #[inline]
    pub fn set_firmware_version_minor(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::FIRMWARE_VERSION_MINOR], value)
    }

    /// Set the firmware version major field.
    #[inline]
    pub fn set_firmware_version_major(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::FIRMWARE_VERSION_MAJOR], value)
    }

    /// Set the device ID field.
    #[inline]
    pub fn set_device_id(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::DEVICE_ID], value)
    }

    /// Set the device serial number word 0 field.
    #[inline]
    pub fn set_device_serial_number_word0(&mut self, value: u32) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u32(&mut data[field::DEVICE_SERIAL_NUMBER0], value)
    }

    /// Set the device serial number word 1 field.
    #[inline]
    pub fn set_device_serial_number_word1(&mut self, value: u32) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u32(&mut data[field::DEVICE_SERIAL_NUMBER1], value)
    }

    /// Set the device serial number word 2 field.
    #[inline]
    pub fn set_device_serial_number_word2(&mut self, value: u32) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u32(&mut data[field::DEVICE_SERIAL_NUMBER2], value)
    }

    /// Set the sequence number field.
    #[inline]
    pub fn set_sequence_number(&mut self, value: u32) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u32(&mut data[field::SEQUENCE_NUMBER], value)
    }

    /// Set the uptime seconds field.
    #[inline]
    pub fn set_uptime_seconds(&mut self, value: u32) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u32(&mut data[field::UPTIME_SECONDS], value)
    }

    /// Set the status flags field.
    #[inline]
    pub fn set_status_flags(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::STATUS_FLAGS], value)
    }

    /// Set the date-time year field.
    #[inline]
    pub fn set_datetime_year(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::DATETIME_YEAR], value)
    }

    /// Set the date-time month field.
    #[inline]
    pub fn set_datetime_month(&mut self, value: u8) {
        let data = self.buffer.as_mut();
        data[field::DATETIME_MONTH] = value;
    }

    /// Set the date-time day field.
    #[inline]
    pub fn set_datetime_day(&mut self, value: u8) {
        let data = self.buffer.as_mut();
        data[field::DATETIME_DAY] = value;
    }

    /// Set the date-time hour field.
    #[inline]
    pub fn set_datetime_hour(&mut self, value: u8) {
        let data = self.buffer.as_mut();
        data[field::DATETIME_HOUR] = value;
    }

    /// Set the date-time minute field.
    #[inline]
    pub fn set_datetime_minute(&mut self, value: u8) {
        let data = self.buffer.as_mut();
        data[field::DATETIME_MINUTE] = value;
    }

    /// Set the date-time second field.
    #[inline]
    pub fn set_datetime_second(&mut self, value: u8) {
        let data = self.buffer.as_mut();
        data[field::DATETIME_SECOND] = value;
    }

    /// Set the temperature field.
    #[inline]
    pub fn set_temperature(&mut self, value: i32) {
        let data = self.buffer.as_mut();
        LittleEndian::write_i32(&mut data[field::TEMPERATURE], value)
    }

    /// Set the humidity field.
    #[inline]
    pub fn set_humidity(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::HUMIDITY], value)
    }

    /// Set the VOC ticks field.
    #[inline]
    pub fn set_voc_ticks(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::VOC_TICKS], value)
    }

    /// Set the NOx ticks field.
    #[inline]
    pub fn set_nox_ticks(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::NOX_TICKS], value)
    }

    /// Set the VOC index field.
    #[inline]
    pub fn set_voc_index(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::VOC_INDEX], value)
    }

    /// Set the NOx index field.
    #[inline]
    pub fn set_nox_index(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::NOX_INDEX], value)
    }

    /// Set the PM2.5 (under atmospheric environment) index field.
    #[inline]
    pub fn set_pm2_5_atm(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::PM2_5_ATM], value)
    }

    /// Set the CO2 index field.
    #[inline]
    pub fn set_co2(&mut self, value: u16) {
        let data = self.buffer.as_mut();
        LittleEndian::write_u16(&mut data[field::CO2], value)
    }

    /// Return a mutable pointer to the remaining data following a message, if any.
    #[inline]
    pub fn rest_mut(&mut self) -> &mut [u8] {
        let data = self.buffer.as_mut();
        &mut data[field::REST]
    }
}

impl<T: AsRef<[u8]>> AsRef<[u8]> for Message<T> {
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_ref()
    }
}

impl<T: AsRef<[u8]>> fmt::Display for Message<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Message proto={}, device={}, status=0x{:04X}",
            self.protocol_version(),
            self.device_id(),
            self.status_flags(),
        )
    }
}

// TODO
// docs on the fields in this struct, about units, etc
// converions methods for raw to scaled
/// A high-level representation of a message.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Repr {
    pub protocol_version: ProtocolVersion,
    pub firmware_version: FirmwareVersion,
    pub device_id: DeviceId,
    pub device_serial_number: DeviceSerialNumber,
    pub sequence_number: u32,
    pub uptime_seconds: u32,
    pub status_flags: StatusFlags,
    pub datetime: DateTime,
    /// The temperature in centidegress C
    pub temperature: i32,
    /// The relative humidity in centipercent
    pub humidity: u16,
    pub voc_ticks: u16,
    pub nox_ticks: u16,
    pub voc_index: u16,
    pub nox_index: u16,
    /// PM2.5 concentration unit μ g/m3（under atmospheric environment）
    pub pm2_5_atm: u16,
    /// CO2 ppm
    pub co2: u16,
}

impl Repr {
    /// Parse a message and return a high-level representation.
    pub fn parse<T: AsRef<[u8]> + ?Sized>(msg: &Message<&T>) -> Result<Repr> {
        msg.check_len()?;
        msg.check_protocol()?;
        Ok(Repr {
            protocol_version: ProtocolVersion(msg.protocol_version()),
            firmware_version: FirmwareVersion {
                major: msg.firmware_version_major(),
                minor: msg.firmware_version_minor(),
                patch: msg.firmware_version_patch(),
            },
            device_id: DeviceId(msg.device_id()),
            device_serial_number: DeviceSerialNumber {
                word0: msg.device_serial_number_word0(),
                word1: msg.device_serial_number_word1(),
                word2: msg.device_serial_number_word2(),
            },
            sequence_number: msg.sequence_number(),
            uptime_seconds: msg.uptime_seconds(),
            status_flags: StatusFlags(msg.status_flags()),
            datetime: DateTime {
                year: msg.datetime_year(),
                month: msg.datetime_month(),
                day: msg.datetime_day(),
                hour: msg.datetime_hour(),
                minute: msg.datetime_minute(),
                second: msg.datetime_second(),
            },
            temperature: msg.temperature(),
            humidity: msg.humidity(),
            voc_ticks: msg.voc_ticks(),
            nox_ticks: msg.nox_ticks(),
            voc_index: msg.voc_index(),
            nox_index: msg.nox_index(),
            pm2_5_atm: msg.pm2_5_atm(),
            co2: msg.co2(),
        })
    }

    /// Return the length of a message that will be emitted from this high-level representation.
    pub const fn message_len(&self) -> usize {
        MESSAGE_LEN
    }

    /// Emit a high-level representation into a message.
    pub fn emit<T: AsRef<[u8]> + AsMut<[u8]>>(&self, msg: &mut Message<T>) {
        msg.set_protocol(ProtocolIdentifier::Broadcast);
        msg.set_protocol_version(self.protocol_version.0);
        msg.set_firmware_version_patch(self.firmware_version.patch);
        msg.set_firmware_version_minor(self.firmware_version.minor);
        msg.set_firmware_version_major(self.firmware_version.major);
        msg.set_device_id(self.device_id.0);
        msg.set_device_serial_number_word0(self.device_serial_number.word0);
        msg.set_device_serial_number_word1(self.device_serial_number.word1);
        msg.set_device_serial_number_word2(self.device_serial_number.word2);
        msg.set_sequence_number(self.sequence_number);
        msg.set_uptime_seconds(self.uptime_seconds);
        msg.set_status_flags(self.status_flags.0);
        msg.set_datetime_year(self.datetime.year);
        msg.set_datetime_month(self.datetime.month);
        msg.set_datetime_day(self.datetime.day);
        msg.set_datetime_hour(self.datetime.hour);
        msg.set_datetime_minute(self.datetime.minute);
        msg.set_datetime_second(self.datetime.second);
        msg.set_temperature(self.temperature);
        msg.set_humidity(self.humidity);
        msg.set_voc_ticks(self.voc_ticks);
        msg.set_nox_ticks(self.nox_ticks);
        msg.set_voc_index(self.voc_index);
        msg.set_nox_index(self.nox_index);
        msg.set_pm2_5_atm(self.pm2_5_atm);
        msg.set_co2(self.co2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static MSG_BYTES: [u8; 60] = [
        0x42, 0x52, 0x44, 0x43, 0x01, 0x03, 0x00, 0x02, 0x00, 0x01, 0x00, 0x0D, 0x00, 0xAA, 0xAA,
        0xAA, 0xAA, 0xBB, 0xBB, 0xBB, 0xBB, 0xCC, 0xCC, 0xCC, 0xCC, 0x01, 0x00, 0x00, 0xFF, 0x44,
        0x33, 0x22, 0x11, 0xBB, 0xAA, 0xE7, 0x07, 0x02, 0x15, 0x10, 0x28, 0x37, 0xEA, 0xFF, 0xFF,
        0xFF, 0xE8, 0x03, 0xAB, 0x00, 0xCD, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
    ];

    #[test]
    fn buffer_too_small() {
        let bytes = [0xFF; 12];
        assert!(Message::new_checked(&bytes[..]).is_err());
        let msg = Message::new_unchecked(&bytes[..]);
        assert_eq!(msg.check_len(), Err(Error));
        assert_eq!(Message::<&[u8]>::message_len(), MESSAGE_LEN);
    }

    #[test]
    fn test_construct() {
        let mut bytes = [0xFF; 60];
        let mut msg = Message::new_unchecked(&mut bytes);
        msg.set_protocol(ProtocolIdentifier::Broadcast);
        msg.set_protocol_version(1);
        msg.set_firmware_version_patch(3);
        msg.set_firmware_version_minor(2);
        msg.set_firmware_version_major(1);
        msg.set_device_id(13);
        msg.set_device_serial_number_word0(0xAAAA_AAAA);
        msg.set_device_serial_number_word1(0xBBBB_BBBB);
        msg.set_device_serial_number_word2(0xCCCC_CCCC);
        msg.set_sequence_number(0xFF00_0001);
        msg.set_uptime_seconds(0x11_22_33_44);
        msg.set_status_flags(0xAA_BB);
        msg.set_datetime_year(2023);
        msg.set_datetime_month(2);
        msg.set_datetime_day(21);
        msg.set_datetime_hour(16);
        msg.set_datetime_minute(40);
        msg.set_datetime_second(55);
        msg.set_temperature(-22);
        msg.set_humidity(1000);
        msg.set_voc_ticks(0xAB);
        msg.set_nox_ticks(0xCD);
        msg.set_voc_index(0x2211);
        msg.set_nox_index(0x4433);
        msg.set_pm2_5_atm(0x6655);
        msg.set_co2(0x8877);
        assert_eq!(msg.into_inner(), &MSG_BYTES[..]);
    }

    #[test]
    fn test_deconstruct() {
        let msg = Message::new_unchecked(&MSG_BYTES[..]);
        assert_eq!(msg.protocol(), ProtocolIdentifier::Broadcast);
        assert_eq!(msg.protocol_version(), 1);
        assert_eq!(msg.firmware_version_patch(), 3);
        assert_eq!(msg.firmware_version_minor(), 2);
        assert_eq!(msg.firmware_version_major(), 1);
        assert_eq!(msg.device_id(), 13);
        assert_eq!(msg.device_serial_number_word0(), 0xAAAA_AAAA);
        assert_eq!(msg.device_serial_number_word1(), 0xBBBB_BBBB);
        assert_eq!(msg.device_serial_number_word2(), 0xCCCC_CCCC);
        assert_eq!(msg.sequence_number(), 0xFF00_0001);
        assert_eq!(msg.uptime_seconds(), 0x11_22_33_44);
        assert_eq!(msg.status_flags(), 0xAA_BB);
        assert_eq!(msg.datetime_year(), 2023);
        assert_eq!(msg.datetime_month(), 2);
        assert_eq!(msg.datetime_day(), 21);
        assert_eq!(msg.datetime_hour(), 16);
        assert_eq!(msg.datetime_minute(), 40);
        assert_eq!(msg.datetime_second(), 55);
        assert_eq!(msg.temperature(), -22);
        assert_eq!(msg.humidity(), 1000);
        assert_eq!(msg.voc_ticks(), 0xAB);
        assert_eq!(msg.nox_ticks(), 0xCD);
        assert_eq!(msg.voc_index(), 0x2211);
        assert_eq!(msg.nox_index(), 0x4433);
        assert_eq!(msg.pm2_5_atm(), 0x6655);
        assert_eq!(msg.co2(), 0x8877);
        let _checked_msg = Message::new_checked(&MSG_BYTES[..]).unwrap();
    }

    #[test]
    fn test_repr_roundtrip() {
        let msg_in = Message::new_checked(&MSG_BYTES[..]).unwrap();
        let repr = Repr::parse(&msg_in).unwrap();
        let mut bytes_out = [0xFF; 60];
        let mut msg_out = Message::new_unchecked(&mut bytes_out);
        repr.emit(&mut msg_out);
        assert_eq!(msg_in.into_inner(), msg_out.into_inner());
    }
}
