use crate::config;
use core::fmt::{self, Write as FmtWrite};
use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use heapless::String;
use sh1106::{prelude::*, Builder};
use smoltcp::wire::{EthernetAddress, Ipv4Address};
use stm32f4xx_hal::hal::blocking::i2c::Write;
use wire_protocols::{DeviceId, DeviceSerialNumber, FirmwareVersion};

const DH: i32 = 64 / 4;
const Y_R0: i32 = 0;
const Y_R1: i32 = DH;
const Y_R2: i32 = 2 * DH;
const Y_R3: i32 = 3 * DH;

const DW: i32 = 128 / 2;
const X_C0: i32 = 0;
const X_C1: i32 = DW;

const STATUS_FONT: MonoFont<'_> = profont::PROFONT_10_POINT;
const INFO_FONT: MonoFont<'_> = profont::PROFONT_9_POINT;

const LINE_BUF_CAP: usize = 128 + 2;

#[derive(Debug)]
pub enum Error<E> {
    Infallible,
    Format,
    Display(sh1106::Error<E, ()>),
}

#[derive(Debug)]
pub struct SystemInfo {
    pub device_id: DeviceId,
    pub firmware_version: FirmwareVersion,
    pub ip: Ipv4Address,
    pub mac: EthernetAddress,
    pub device_serial_number: DeviceSerialNumber,
}

impl SystemInfo {
    pub const fn new() -> Self {
        Self {
            device_id: config::DEVICE_ID,
            firmware_version: config::FIRMWARE_VERSION,
            ip: config::SRC_IP_CIDR.address(),
            mac: EthernetAddress(config::SRC_MAC),
            device_serial_number: DeviceSerialNumber::zero(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default)]
pub struct SystemStatus {
    pub pm2_5: Option<u16>,
    pub co2: Option<u16>,
    pub temp: Option<i32>,
    pub humidity: Option<u16>,
    pub voc_index: Option<u16>,
    pub nox_index: Option<u16>,
    pub msg_seqnum: u32,
}

impl SystemStatus {
    pub const fn new() -> Self {
        Self {
            pm2_5: None,
            co2: None,
            temp: None,
            humidity: None,
            voc_index: None,
            nox_index: None,
            msg_seqnum: 0,
        }
    }

    fn aqi(&self) -> Option<u16> {
        self.pm2_5
            .and_then(|concentration| aqi::pm2_5(f64::from(concentration.clamp(0, 500))).ok())
            .map(|aqi| aqi.aqi() as u16)
    }

    fn temp_c(&self) -> Option<f32> {
        self.temp
            .map(f64::from)
            .map(|centi_deg| centi_deg as f32 / 100.0)
    }

    fn temp_f(&self) -> Option<f32> {
        self.temp_c().map(|c| (c * 1.8) + 32.0)
    }

    fn rel_humidity(&self) -> Option<f32> {
        self.humidity
            .map(f32::from)
            .map(|centi_rh| centi_rh / 100.0)
    }
}

pub struct Display<I2C>
where
    I2C: Write,
{
    drv: GraphicsMode<I2cInterface<I2C>>,
    line_buf: String<LINE_BUF_CAP>,
}

impl<I2C, E> Display<I2C>
where
    I2C: Write<Error = E>,
{
    pub fn new(i2c: I2C) -> Result<Self, Error<E>> {
        let mut drv: GraphicsMode<_> = Builder::new()
            .with_i2c_addr(0x3C)
            .with_size(DisplaySize::Display128x64)
            .connect_i2c(i2c)
            .into();

        drv.init()?;
        drv.set_contrast(0xFF)?;
        drv.clear();
        drv.flush()?;

        Ok(Display {
            drv,
            line_buf: String::new(),
        })
    }

    pub fn render_system_info(&mut self, view: &SystemInfo) -> Result<(), Error<E>> {
        let text_style = MonoTextStyleBuilder::new()
            .font(&INFO_FONT)
            .text_color(BinaryColor::On)
            .build();

        self.drv.clear();

        self.line_buf.clear();
        write!(&mut self.line_buf, "ID: {}", view.device_id)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C0, Y_R0),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        self.line_buf.clear();
        write!(&mut self.line_buf, "V: {}", view.firmware_version)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C1, Y_R0),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        self.line_buf.clear();
        write!(&mut self.line_buf, "IP: {}", view.ip)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C0, Y_R1),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        self.line_buf.clear();
        write!(&mut self.line_buf, "M: {}", view.mac)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C0, Y_R2),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        self.line_buf.clear();
        write!(&mut self.line_buf, "{:X}", view.device_serial_number)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C0, Y_R3),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        self.drv.flush()?;

        Ok(())
    }

    pub fn render_system_status(&mut self, view: &SystemStatus) -> Result<(), Error<E>> {
        let text_style = MonoTextStyleBuilder::new()
            .font(&STATUS_FONT)
            .text_color(BinaryColor::On)
            .build();

        self.drv.clear();

        let val = view.aqi();
        let val = DisplayOption(&val);
        self.line_buf.clear();
        write!(&mut self.line_buf, "AQI: {}", val)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C0, Y_R0),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        let val = DisplayOption(&view.co2);
        self.line_buf.clear();
        write!(&mut self.line_buf, "CO2: {}", val)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C1, Y_R0),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        let val = DisplayOption(&view.voc_index);
        self.line_buf.clear();
        write!(&mut self.line_buf, "VOC: {}", val)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C0, Y_R1),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        let val = DisplayOption(&view.nox_index);
        self.line_buf.clear();
        write!(&mut self.line_buf, "NOX: {}", val)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C1, Y_R1),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        let val = view.temp_f();
        let val = DisplayOption(&val);
        self.line_buf.clear();
        write!(&mut self.line_buf, "F: {}", val)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C0, Y_R2),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        let val = view.rel_humidity();
        let val = DisplayOption(&val);
        self.line_buf.clear();
        write!(&mut self.line_buf, "H: {}", val)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C1, Y_R2),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        self.line_buf.clear();
        write!(&mut self.line_buf, "MSG: {}", view.msg_seqnum)?;
        Text::with_baseline(
            self.line_buf.as_str(),
            Point::new(X_C0, Y_R3),
            text_style,
            Baseline::Top,
        )
        .draw(&mut self.drv)?;

        self.drv.flush()?;

        Ok(())
    }
}

// TODO - this could use some cleanup
#[repr(transparent)]
struct DisplayOption<'a, T>(pub &'a Option<T>);

impl<'a> fmt::Display for DisplayOption<'a, u16> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            None => f.write_str("NA"),
            Some(v) => v.fmt(f),
        }
    }
}

impl<'a> fmt::Display for DisplayOption<'a, f32> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            None => f.write_str("NA"),
            Some(v) => write!(f, "{v:.02}"),
        }
    }
}

impl<E> From<sh1106::Error<E, ()>> for Error<E> {
    fn from(value: sh1106::Error<E, ()>) -> Self {
        Error::Display(value)
    }
}

impl<E> From<fmt::Error> for Error<E> {
    fn from(_value: fmt::Error) -> Self {
        Error::Format
    }
}

impl<E> From<core::convert::Infallible> for Error<E> {
    fn from(_value: core::convert::Infallible) -> Self {
        Error::Infallible
    }
}
