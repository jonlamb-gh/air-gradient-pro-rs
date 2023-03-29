// TODO
// split rx/tx
// make rx int driven
// isr feeds into task or this module
//
// or just use blocking mode, doesn't have that much traffic...
// https://github.com/g-bartoszek/pms-7003
//
// the task will put the device in sleep mode for some time
// then wake and measure
// periodically
//
// probably use passive mode
//
// https://forum.airgradient.com/t/extending-the-life-span-of-the-pms5003-sensor/114
//
// https://github.com/airgradienthq/arduino/blob/43f599a0a7d65524c49d00f546f814420aeaed6e/AirGradient.cpp#L251

use pms_7003::{Error, OutputFrame, Pms7003Sensor};
use stm32f4xx_hal::{
    gpio::{PushPull, AF7, PA2, PA3},
    hal::blocking::delay::DelayMs,
    hal::serial::{Read, Write},
    pac::USART2,
    serial::Serial,
};

pub type DefaultPms5003Serial = Serial<USART2, (PA2<AF7<PushPull>>, PA3<AF7<PushPull>>)>;

pub struct Pms5003<Serial = DefaultPms5003Serial>
where
    Serial: Read<u8> + Write<u8>,
{
    drv: Pms7003Sensor<Serial>,
}

impl<Serial> Pms5003<Serial>
where
    Serial: Read<u8> + Write<u8>,
{
    pub fn new<D: DelayMs<u8>>(serial: Serial, delay: &mut D) -> Result<Self, Error> {
        let mut drv = Pms7003Sensor::new(serial);
        // Default mode after power up is active mode
        // Wake up and read to flush the line before
        // changing modes and sleeping in case
        // it was just a reboot not power cycle
        // the pms_7003 lib only works this way currently
        drv.wake()?;
        delay.delay_ms(100_u8);
        let _ = drv.read()?;

        log::info!("PMS5003: entering standy mode");
        drv.passive()?;
        delay.delay_ms(100_u8);
        drv.sleep()?;

        Ok(Self { drv })
    }
}
