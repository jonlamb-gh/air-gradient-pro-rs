use core::fmt;
use pms_7003::{Error, Pms7003Sensor};
use stm32f4xx_hal::{
    gpio::{PushPull, AF7, PA2, PA3},
    hal::blocking::delay::DelayMs,
    hal::serial::{Read, Write},
    pac::USART2,
    serial::Serial,
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Measurement {
    /// PM2.5 concentration unit μ g/m3 (under atmospheric environment)
    pub pm2_5_atm: u16,
}

pub type Pms5003SerialPins = (PA2<AF7<PushPull>>, PA3<AF7<PushPull>>);
pub type DefaultPms5003Serial = Serial<USART2>;

pub struct Pms5003<Serial = DefaultPms5003Serial>
where
    Serial: Read<u8> + Write<u8>,
{
    drv: Pms7003Sensor<Serial>,
}

impl<Serial> Pms5003<Serial>
where
    Serial: Read<u8> + Write<u8>,
{
    pub fn new<D: DelayMs<u8>>(serial: Serial, delay: &mut D) -> Result<Self, Error> {
        let mut drv = Pms7003Sensor::new(serial);
        // Default mode after power up is active mode
        // Wake up and read to flush the line before
        // changing modes and sleeping in case
        // it was just a reboot not power cycle
        // the pms_7003 lib only works this way currently
        drv.wake()?;
        delay.delay_ms(100_u8);
        let _ = drv.read()?;

        log::debug!("PMS5003: entering standby mode");
        drv.passive()?;
        delay.delay_ms(100_u8);
        let mut pms = Self { drv };
        pms.enter_standby_mode()?;

        Ok(pms)
    }

    pub fn enter_standby_mode(&mut self) -> Result<(), Error> {
        self.drv.sleep()?;
        Ok(())
    }

    // NOTE: the sensor wakes up from sleep in active mode
    pub fn enter_active_mode(&mut self) -> Result<(), Error> {
        self.drv.wake()?;
        Ok(())
    }

    pub fn measure(&mut self) -> Result<Measurement, Error> {
        let f = self.drv.read()?;
        Ok(Measurement {
            pm2_5_atm: f.pm2_5_atm,
        })
    }
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PMS5003 pm2_5_atm: {}", self.pm2_5_atm)
    }
}
