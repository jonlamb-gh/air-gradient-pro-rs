use embedded_graphics::{
    mock_display::MockDisplay,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
    text::{Alignment, Text},
};
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

        // TODO testing
        let border_stroke = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(8)
            .stroke_alignment(StrokeAlignment::Inside)
            .build();
        drv.bounding_box()
            .into_styled(border_stroke)
            .draw(&mut drv)
            .unwrap();

        Ok(Display { drv })
    }
}
