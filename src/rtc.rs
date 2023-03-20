use ds323x::{ic::DS3231, interface::I2cInterface, DateTimeAccess, Ds323x, Error, NaiveDateTime};
use stm32f4xx_hal::{
    gpio::{OpenDrain, AF4, PB6, PB7},
    hal::blocking::i2c::{Write, WriteRead},
    i2c::I2c,
    pac::I2C1,
};

pub type DefaultRtcI2c = I2c<I2C1, (PB6<AF4<OpenDrain>>, PB7<AF4<OpenDrain>>)>;

pub struct Rtc<I2C = DefaultRtcI2c> {
    drv: Ds323x<I2cInterface<I2C>, DS3231>,
}

impl<I2C, E> Rtc<I2C>
where
    I2C: Write<Error = E> + WriteRead<Error = E>,
{
    pub fn new(i2c: I2C) -> Result<Self, Error<E, ()>> {
        let mut drv = Ds323x::new_ds3231(i2c);
        drv.disable_square_wave()?;
        drv.disable_32khz_output()?;
        drv.disable_alarm1_interrupts()?;
        drv.disable_alarm2_interrupts()?;
        drv.enable()?;
        Ok(Rtc { drv })
    }

    pub fn datetime(&mut self) -> Result<NaiveDateTime, Error<E, ()>> {
        self.drv.datetime()
    }

    pub fn set_datetime(&mut self, dt: &NaiveDateTime) -> Result<(), Error<E, ()>> {
        self.drv.set_datetime(dt)
    }

    #[cfg(test)]
    pub fn release(self) -> I2C {
        self.drv.destroy_ds3231()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_runner::TestResources;
    use ds323x::NaiveDate;
    use stm32f4xx_hal::prelude::*;

    #[test_case]
    fn roundtrip_datetime(res: TestResources) {
        let gpiob = res.dp.GPIOB.split();
        let scl = gpiob.pb8.into_alternate().set_open_drain();
        let sda = gpiob.pb9.into_alternate().set_open_drain();
        let i2c = res.dp.I2C1.i2c((scl, sda), 100.kHz(), &res.clocks);
        let mut rtc = Rtc::new(i2c).unwrap();
        let new_dt = NaiveDate::from_ymd_opt(2020, 5, 1)
            .unwrap()
            .and_hms_opt(19, 59, 30)
            .unwrap();
        rtc.set_datetime(&new_dt).unwrap();
        let now = rtc.datetime().unwrap();
        assert!(now >= new_dt);

        let i2c = rtc.release();
        let (_i2c, _pins) = i2c.release();
    }
}
