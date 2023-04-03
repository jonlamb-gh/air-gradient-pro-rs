use influxdb2::models::{data_point::DataPointError, DataPoint};
use wire_protocols::broadcast::Repr as Message;

#[derive(Debug, PartialEq, Clone)]
pub struct Measurement {
    pub recv_time_utc_ns: i64,
    pub tags: MeasurementTags,
    pub fields: MeasurementFields,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MeasurementTags {
    pub device_id: String,
    pub device_serial_number: String,
    pub firmware_version: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MeasurementFields {
    pub sequence_number: i64,
    pub temperature: Option<f64>,
    pub humidity: Option<f64>,
    pub voc_ticks: Option<i64>,
    pub nox_ticks: Option<i64>,
    pub voc_index: Option<i64>,
    pub nox_index: Option<i64>,
    pub pm25: Option<i64>,
    pub aqi: Option<i64>,
    pub aqi_level: Option<String>,
    pub co2: Option<i64>,
}

impl Measurement {
    pub fn into_data_point(self) -> Result<DataPoint, DataPointError> {
        let mut dpb = DataPoint::builder("measurement")
            .timestamp(self.recv_time_utc_ns)
            .tag("device_id", self.tags.device_id)
            .tag("device_serial_number", self.tags.device_serial_number)
            .tag("firmware_version", self.tags.firmware_version)
            .field("sequence_number", self.fields.sequence_number);
        if let Some(v) = self.fields.temperature {
            dpb = dpb.field("temperature", v);
        }
        if let Some(v) = self.fields.humidity {
            dpb = dpb.field("humidity", v);
        }
        if let Some(v) = self.fields.voc_ticks {
            dpb = dpb.field("voc_ticks", v);
        }
        if let Some(v) = self.fields.nox_ticks {
            dpb = dpb.field("nox_ticks", v);
        }
        if let Some(v) = self.fields.voc_index {
            dpb = dpb.field("voc_index", v);
        }
        if let Some(v) = self.fields.nox_index {
            dpb = dpb.field("nox_index", v);
        }
        if let Some(v) = self.fields.pm25 {
            dpb = dpb.field("pm25", v);
        }
        if let Some(v) = self.fields.aqi {
            dpb = dpb.field("aqi", v);
        }
        if let Some(v) = self.fields.aqi_level {
            dpb = dpb.field("aqi_level", v);
        }
        if let Some(v) = self.fields.co2 {
            dpb = dpb.field("co2", v);
        }
        dpb.build()
    }
}

pub trait MessageExt {
    fn temperature_c(&self) -> f64;

    fn temperature_f(&self) -> f64 {
        (self.temperature_c() * 1.8) + 32.0
    }

    fn relative_humidity(&self) -> f64;

    fn pm2_5_us_aqi(&self) -> aqi::AirQuality;
}

impl MessageExt for Message {
    fn temperature_c(&self) -> f64 {
        f64::from(self.temperature) / 100.0
    }

    fn relative_humidity(&self) -> f64 {
        f64::from(self.humidity) / 100.0
    }

    fn pm2_5_us_aqi(&self) -> aqi::AirQuality {
        // Should already be clamped
        let concentration = self.pm2_5_atm.clamp(0, 500);
        aqi::pm2_5(f64::from(concentration)).expect("PM2.5 concentration out of range")
    }
}
