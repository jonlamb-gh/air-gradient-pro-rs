use crate::firmware_main::app::data_manager_task;
use crate::sensors::{sgp41, sht31};
use log::info;
use stm32f4xx_hal::prelude::*;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum SpawnArg {
    /// Temperature and humidity measurement from the SHT31 sensor
    Sht31Measurement(sht31::Measurement),
    /// VOC and NOx measurement from the SGP41 sensor
    Sgp41Measurement(sgp41::Measurement),
    /// Time to send the data
    SendData,
    // TODO
    // - udp send deadline reached/timer stuff
}

// TODO - takes an enum arg, each sensor task sends/spawns this task
pub(crate) fn data_manager_task(ctx: data_manager_task::Context, arg: SpawnArg) {
    // TODO
    info!("Data manager task updating reason={}", arg.as_reason());

    if matches!(arg, SpawnArg::SendData) {
        // send data period
        data_manager_task::spawn_after(2.secs(), SpawnArg::SendData).unwrap();
    }
}

impl SpawnArg {
    fn as_reason(&self) -> &'static str {
        match self {
            SpawnArg::Sht31Measurement(_) => "sht31-measurement",
            SpawnArg::Sgp41Measurement(_) => "sgp41-measurement",
            SpawnArg::SendData => "send-data",
        }
    }
}
