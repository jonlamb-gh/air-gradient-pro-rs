use sh1106::{prelude::*, Builder, Error};
use stm32f4xx_hal::hal::blocking::i2c::Write;

pub struct Display<I2C>
where
    I2C: Write,
{
    drv: GraphicsMode<I2cInterface<I2C>>,
}

impl<I2C, E> Display<I2C>
where
    I2C: Write<Error = E>,
{
    pub fn new(i2c: I2C) -> Result<Self, Error<E, ()>> {
        let mut drv: GraphicsMode<_> = Builder::new()
            .with_i2c_addr(0x3C)
            .with_size(DisplaySize::Display128x64)
            .connect_i2c(i2c)
            .into();
        drv.init()?;
        drv.clear();
        drv.flush()?;
        Ok(Display { drv })
    }
}
