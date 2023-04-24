use crate::display::Display;
use crate::sensors::Sgp41;
use crate::sensors::Sht31;
use shared_bus::AtomicCheckMutex;
use stm32f4xx_hal::{
    gpio::{OpenDrain, AF4, AF9, PB10, PB3},
    hal::blocking::i2c::Write,
    pac::I2C2,
};

pub type I2cPins = (PB10<AF4<OpenDrain>>, PB3<AF9<OpenDrain>>);
pub type I2c<PINS = I2cPins> = stm32f4xx_hal::i2c::I2c<I2C2, PINS>;
pub type I2cProxy<I2C> = shared_bus::I2cProxy<'static, AtomicCheckMutex<I2C>>;

pub struct I2cDevices<D0, D1, I2C = I2c<I2cPins>>
where
    I2C: Write + 'static,
{
    pub display: Display<I2cProxy<I2C>>,
    pub sht31: Sht31<I2cProxy<I2C>, D0>,
    pub sgp41: Sgp41<I2cProxy<I2C>, D1>,
}
