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

pub struct Pms5003<D, Serial = DefaultPms5003Serial>
where
    D: DelayMs<u8>,
    Serial: Read<u8> + Write<u8>,
{
    drv: Pms7003Sensor<Serial>,
    delay: D,
}

impl<D, Serial> Pms5003<D, Serial>
where
    D: DelayMs<u8>,
    Serial: Read<u8> + Write<u8>,
{
    pub fn new(serial: Serial, mut delay: D) -> Result<Self, Error> {
        let mut drv = Pms7003Sensor::new(serial);
        // Default mode after power up is active mode
        // Wake up and read to flush the line before
        // changing modes and sleeping in case
        // it was just a reboot not power cycle
        // the pms_7003 lib only works this way currently
        drv.wake()?;
        delay.delay_ms(100_u8);

        let _ = drv.read();
        drv.passive()?;

        log::debug!("PMS5003: entering standby mode");
        let mut pms = Self { drv, delay };
        pms.enter_standby_mode()?;

        Ok(pms)
    }

    pub fn enter_standby_mode(&mut self) -> Result<(), Error> {
        self.delay.delay_ms(100_u8);
        self.drv.sleep()?;
        Ok(())
    }

    // NOTE: the sensor wakes up from sleep in active mode
    pub fn enter_ready_mode(&mut self) -> Result<(), Error> {
        self.drv.wake()?;
        let _ = self.drv.read();
        self.drv.passive()?;
        Ok(())
    }

    pub fn measure(&mut self) -> Result<Measurement, Error> {
        self.drv.request()?;
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
