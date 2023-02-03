use ds323x::{ic::DS3231, interface::I2cInterface, DateTimeAccess, Ds323x, Error, NaiveDateTime};
use stm32f4xx_hal::{
    gpio::{OpenDrain, AF4, PB8, PB9},
    hal::blocking::i2c::{Write, WriteRead},
    i2c::I2c,
    pac::I2C1,
};

pub type DefaultRtcI2c = I2c<I2C1, (PB8<AF4<OpenDrain>>, PB9<AF4<OpenDrain>>)>;

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
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn simple_test() {
        assert_eq!(1, 1);
    }

    #[test_case]
    fn simple_test2() {
        assert_eq!(1, 2);
    }
}
