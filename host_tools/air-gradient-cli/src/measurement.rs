use influxdb2_derive::WriteDataPoint;
use wire_protocols::broadcast::Repr as Message;

#[derive(Debug, PartialEq, Clone, WriteDataPoint)]
pub struct Measurement {
    #[influxdb(timestamp)]
    pub recv_time_utc_ns: i64,
    #[influxdb(tag)]
    pub device_id: String,
    #[influxdb(tag)]
    pub device_serial_number: String,
    #[influxdb(tag)]
    pub firmware_version: String,
    #[influxdb(field)]
    pub sequence_number: u64,
    #[influxdb(field)]
    pub temperature: Option<f64>,
    #[influxdb(field)]
    pub humidity: Option<f64>,
    #[influxdb(field)]
    pub voc_ticks: Option<u64>,
    #[influxdb(field)]
    pub nox_ticks: Option<u64>,
    #[influxdb(field)]
    pub pm25: Option<u64>,
    #[influxdb(field)]
    pub aqi: Option<u64>,
    #[influxdb(field)]
    pub aqi_level: Option<String>,
    #[influxdb(field)]
    pub co2: Option<u64>,
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
