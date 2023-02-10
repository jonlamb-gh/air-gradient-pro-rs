use sgp41::error::Error;
use stm32f4xx_hal::hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write, WriteRead},
};

pub struct Sgp41<I2C, D> {
    drv: sgp41::sgp41::Sgp41<I2C, D>,
}

impl<I2C, D, E> Sgp41<I2C, D>
where
    I2C: Read<Error = E> + Write<Error = E> + WriteRead<Error = E>,
    D: DelayMs<u32>,
{
    pub fn new(i2c: I2C, delay: D) -> Result<Self, Error<E>> {
        let mut drv = sgp41::sgp41::Sgp41::new(i2c, delay);
        drv.soft_reset()?;

        // TODO
        // self test here or as methods...

        Ok(Sgp41 { drv })
    }

    // TODO - figure out what methods needed
    pub fn self_test(&mut self) -> Result<(), Error<E>> {
        // TODO why is this not using Command::ExecuteSelfTest ??
        self.drv.execute_self_test()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_runner::TestResources;
    use stm32f4xx_hal::prelude::*;

    #[test_case]
    fn sgp41_self_test_and_measurement(res: TestResources) {
        let gpiof = res.dp.GPIOF.split();
        let scl = gpiof.pf1.into_alternate().set_open_drain();
        let sda = gpiof.pf0.into_alternate().set_open_drain();
        let i2c = res.dp.I2C2.i2c((scl, sda), 100.kHz(), &res.clocks);
        let delay = res.cp.SYST.delay(&res.clocks);
        let mut sensor = Sgp41::new(i2c, delay).unwrap();
        // TODO - figure out what methods needed
        let sn = sensor.drv.get_serial_number().unwrap();
        sensor.self_test().unwrap();
        sensor.drv.execute_conditioning().unwrap();
        let raw = sensor.drv.measure_raw().unwrap();
        let compd = sensor.drv.measure_raw_compensated(10, 20).unwrap();
        panic!("sn=0x{sn:X}\n{raw:#?}\n{compd:#?}");
    }
}
