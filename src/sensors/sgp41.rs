use crate::sensors::sht31;
use core::fmt;
use sgp41::error::Error;
use stm32f4xx_hal::hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write, WriteRead},
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Measurement {
    pub voc_ticks: u16,
    pub nox_ticks: u16,
}

pub const fn default_compensation() -> sht31::RawMeasurement {
    sht31::RawMeasurement {
        humidity_ticks: 0x8000,
        temperature_ticks: 0x6666,
    }
}

pub struct Sgp41<I2C, D> {
    sn: u64,
    drv: sgp41::sgp41::Sgp41<I2C, D>,
}

impl<I2C, D, E> Sgp41<I2C, D>
where
    I2C: Read<Error = E> + Write<Error = E> + WriteRead<Error = E>,
    D: DelayMs<u32>,
{
    pub fn new(i2c: I2C, delay: D) -> Result<Self, Error<E>> {
        let mut drv = sgp41::sgp41::Sgp41::new(i2c, delay);
        drv.turn_heater_off()?;
        drv.execute_self_test()?;
        let sn = drv.get_serial_number()?;
        Ok(Sgp41 { sn, drv })
    }

    pub fn serial_number(&self) -> u64 {
        self.sn
    }

    pub fn execute_conditioning(&mut self) -> Result<(), Error<E>> {
        let _raw_voc = self.drv.execute_conditioning()?;
        Ok(())
    }

    pub fn measure(
        &mut self,
        compensation: &sht31::RawMeasurement,
    ) -> Result<Measurement, Error<E>> {
        self.drv
            .measure_raw_compensated(compensation.humidity_ticks, compensation.temperature_ticks)
            .map(Measurement::from)
    }
}

impl From<sgp41::types::RawSensorData> for Measurement {
    fn from(value: sgp41::types::RawSensorData) -> Self {
        Self {
            voc_ticks: value.voc_ticks,
            nox_ticks: value.nox_ticks,
        }
    }
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SGP41 voc_ticks: {}, nox_ticks: {}",
            self.voc_ticks, self.nox_ticks,
        )
    }
}
