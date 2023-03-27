use core::fmt;
use sht3x::{Address, ClockStretch, Error, Rate, Repeatability, Sht3x};
use stm32f4xx_hal::hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write, WriteRead},
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Measurement {
    /// The temperature in millidegress C
    pub temperature: i32,
    /// The relative humidity in millipercent
    pub humidity: u16,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct RawMeasurement {
    pub temperature_ticks: u16,
    pub humidity_ticks: u16,
}

pub struct Sht31<I2C, D> {
    sn: u16,
    drv: Sht3x<I2C>,
    delay: D,
}

impl<I2C, D, E> Sht31<I2C, D>
where
    I2C: Read<Error = E> + Write<Error = E> + WriteRead<Error = E>,
    D: DelayMs<u8>,
{
    pub fn new(i2c: I2C, mut delay: D) -> Result<Self, Error<E>> {
        // TODO - do what the arduino lib does for configs and startup procedure
        // periodicStart(SHT3XD_REPEATABILITY_HIGH, SHT3XD_FREQUENCY_10HZ);
        // read sn
        // tempHumInterval = 2500 ms
        // periodicFetchData in the loop
        //   SHT3XD_CMD_FETCH_DATA
        //   readTemperatureAndHumidity
        let mut drv = Sht3x::new(i2c, Address::Low);
        drv.stop(&mut delay)?;
        delay.delay_ms(20);
        drv.clear_status(&mut delay)?;
        let sn = drv.serial_number(&mut delay)?;
        drv.start(Repeatability::High, Rate::R10, &mut delay)?;
        Ok(Sht31 { sn, drv, delay })
    }

    pub fn serial_number(&self) -> u16 {
        self.sn
    }

    pub fn measure(&mut self) -> Result<(RawMeasurement, Measurement), Error<E>> {
        self.drv
            .fetch_data(&mut self.delay)
            .map(|m| (RawMeasurement::from(&m), Measurement::from(&m)))
    }
}

impl From<&sht3x::Measurement> for Measurement {
    fn from(value: &sht3x::Measurement) -> Self {
        Self {
            temperature: value.temperature,
            humidity: value.humidity,
        }
    }
}

impl From<&sht3x::Measurement> for RawMeasurement {
    fn from(value: &sht3x::Measurement) -> Self {
        Self {
            temperature_ticks: value.raw_temperature,
            humidity_ticks: value.raw_humidity,
        }
    }
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SHT31 temperature: {}, humidity: {}",
            self.temperature, self.humidity
        )
    }
}
