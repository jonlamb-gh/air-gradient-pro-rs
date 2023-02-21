use crate::display::Display;
use crate::sensors::Sgp41;
use crate::sensors::Sht31;
use shared_bus::AtomicCheckMutex;
use stm32f4xx_hal::{
    gpio::{OpenDrain, AF4, PF0, PF1},
    hal::blocking::i2c::Write,
    pac::I2C2,
};

pub type I2cPins = (PF1<AF4<OpenDrain>>, PF0<AF4<OpenDrain>>);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_runner::TestResources;
    use stm32f4xx_hal::prelude::*;

    #[test_case]
    fn shared_i2c_smoke(res: TestResources) {
        let bus_manager: &'static _ = {
            let gpiof = res.dp.GPIOF.split();
            let scl = gpiof.pf1.into_alternate().set_open_drain();
            let sda = gpiof.pf0.into_alternate().set_open_drain();
            let i2c = res.dp.I2C2.i2c((scl, sda), 100.kHz(), &res.clocks);
            shared_bus::new_atomic_check!(I2c = i2c).unwrap()
        };

        let _dummy_a: I2cProxy<_> = bus_manager.acquire_i2c();
        let _dummy_b: I2cProxy<_> = bus_manager.acquire_i2c();
    }
}
