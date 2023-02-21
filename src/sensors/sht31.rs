use core::fmt;
use sht3x::{Address, ClockStretch, Error, Repeatability, Sht3x};
use stm32f4xx_hal::hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write, WriteRead},
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Measurement {
    pub temperature: i32,
    pub humidity: u16,
}

pub struct Sht31<I2C, D> {
    drv: Sht3x<I2C>,
    delay: D,
}

impl<I2C, D> Sht31<I2C, D> {
    pub const MEASUREMENT_PERIOD_MS: u32 = 1000;
}

impl<I2C, D, E> Sht31<I2C, D>
where
    I2C: Read<Error = E> + Write<Error = E> + WriteRead<Error = E>,
    D: DelayMs<u8>,
{
    pub fn new(i2c: I2C, mut delay: D) -> Result<Self, Error<E>> {
        // TODO - figure out what the air-gradient-pro Arduino firmware does
        // low == address 0x44, high == 0x45
        let mut drv = Sht3x::new(i2c, Address::Low);

        // TODO
        // self test here or as methods...
        drv.reset(&mut delay)?;

        Ok(Sht31 { drv, delay })
    }

    pub fn measure(&mut self) -> Result<Measurement, Error<E>> {
        self.drv
            .measure(ClockStretch::Disabled, Repeatability::High, &mut self.delay)
            .map(Measurement::from)
    }
}

impl From<sht3x::Measurement> for Measurement {
    fn from(value: sht3x::Measurement) -> Self {
        Self {
            temperature: value.temperature,
            humidity: value.humidity,
        }
    }
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SHT31 Measurement temperature: {}, humidity: {}",
            self.temperature, self.humidity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_runner::TestResources;
    use stm32f4xx_hal::prelude::*;

    #[test_case]
    fn sht31_measurement(res: TestResources) {
        let gpiof = res.dp.GPIOF.split();
        let scl = gpiof.pf1.into_alternate().set_open_drain();
        let sda = gpiof.pf0.into_alternate().set_open_drain();
        let i2c = res.dp.I2C2.i2c((scl, sda), 100.kHz(), &res.clocks);
        let delay = res.cp.SYST.delay(&res.clocks);
        let mut sensor = Sht31::new(i2c, delay).unwrap();
        // TODO - figure out what methods needed
        let pre_status = sensor.drv.status(&mut sensor.delay).unwrap();
        let m = sensor.measure().unwrap();
        let post_status = sensor.drv.status(&mut sensor.delay).unwrap();
        //panic!("pre_status = {pre_status:#?}\n{m:#?}\npost_status = {post_status:#?}");
    }
}
