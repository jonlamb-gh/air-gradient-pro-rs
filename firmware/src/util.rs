use wire_protocols::DeviceSerialNumber;

pub(crate) fn read_device_serial_number() -> DeviceSerialNumber {
    let word0 = unsafe { *(0x1FFF_7A10 as *const u32) };
    let word1 = unsafe { *(0x1FFF_7A14 as *const u32) };
    let word2 = unsafe { *(0x1FFF_7A18 as *const u32) };
    DeviceSerialNumber::new(word0, word1, word2)
}
