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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_runner::TestResources;
    use embedded_graphics::{
        image::{Image, ImageRawLE},
        pixelcolor::BinaryColor,
        prelude::*,
    };
    use stm32f4xx_hal::prelude::*;

    #[test_case]
    fn display_drawing(res: TestResources) {
        let gpiof = res.dp.GPIOF.split();
        let scl = gpiof.pf1.into_alternate().set_open_drain();
        let sda = gpiof.pf0.into_alternate().set_open_drain();
        let i2c = res.dp.I2C2.i2c((scl, sda), 100.kHz(), &res.clocks);
        let mut display = Display::new(i2c).unwrap();

        let im: ImageRawLE<BinaryColor> =
            ImageRawLE::new(include_bytes!("../test_resources/rust.raw"), 64);
        Image::new(&im, Point::new(128 / 4, 0))
            .draw(&mut display.drv)
            .unwrap();
        display.drv.flush().unwrap();
    }
}
