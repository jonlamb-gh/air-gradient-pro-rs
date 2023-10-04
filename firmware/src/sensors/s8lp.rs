use core::fmt;
use log::error;
use stm32f4xx_hal::{
    gpio::{PushPull, AF7, PA10, PA9},
    hal::blocking::serial::Write,
    hal::serial::Read,
    nb::block,
    pac::USART1,
    serial,
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Measurement {
    /// CO2 ppm
    pub co2: u16,
}

#[derive(Debug)]
pub enum Error<E> {
    /// Bad response
    Response,
    Serial(E),
}

pub type S8LpSerialPins = (PA9<AF7<PushPull>>, PA10<AF7<PushPull>>);
pub type DefaultS8LpSerial = serial::Serial<USART1>;

pub struct S8Lp<Serial = DefaultS8LpSerial>
where
    Serial: Read<u8> + Write<u8>,
{
    resp_buffer: [u8; 7],
    serial: Serial,
}

impl<E, Serial> S8Lp<Serial>
where
    Serial: Read<u8, Error = E> + Write<u8, Error = E>,
    E: core::fmt::Debug,
{
    const CMD: &'static [u8] = &[0xFE, 0x04, 0x00, 0x03, 0x00, 0x01, 0xD5, 0xC5];

    pub fn new(serial: Serial) -> Self {
        Self {
            resp_buffer: [0; 7],
            serial,
        }
    }

    pub fn measure(&mut self) -> Result<Measurement, Error<E>> {
        self.serial.bwrite_all(Self::CMD)?;

        // TODO - add some frame sync logic
        for idx in 0..self.resp_buffer.len() {
            self.resp_buffer[idx] = block!(self.serial.read())?;
            self.serial.bflush()?;
        }

        // TODO - surface these error variants
        if self.resp_buffer[0] != 0xFE {
            error!("S8LP: bad address");
            Err(Error::Response)
        } else if self.resp_buffer[1] != 0x04 {
            error!("S8LP: bad function code");
            Err(Error::Response)
        } else if self.resp_buffer[2] != 0x02 {
            error!("S8LP: bad payload length");
            Err(Error::Response)
        } else {
            let crc = u16::from_le_bytes([self.resp_buffer[5], self.resp_buffer[6]]);
            let expected_crc = crc16(&self.resp_buffer, self.resp_buffer.len() as u8 - 2);

            if crc != expected_crc {
                error!("S8LP: bad CRC");
                Err(Error::Response)
            } else {
                let payload = u16::from_be_bytes([self.resp_buffer[3], self.resp_buffer[4]]);
                Ok(Measurement { co2: payload })
            }
        }
    }
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "S8LP CO2: {}", self.co2)
    }
}

impl<E> From<E> for Error<E> {
    fn from(value: E) -> Self {
        Error::Serial(value)
    }
}

fn crc16(frame: &[u8], data_length: u8) -> u16 {
    let mut crc: u16 = 0xffff;
    for i in frame.iter().take(data_length as usize) {
        crc ^= u16::from(*i);
        for _ in (0..8).rev() {
            if (crc & 0x0001) == 0 {
                crc >>= 1;
            } else {
                crc >>= 1;
                crc ^= 0xA001;
            }
        }
    }
    crc
}
