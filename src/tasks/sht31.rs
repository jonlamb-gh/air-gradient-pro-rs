use crate::{
    app::{data_manager_task, sgp41_task, sht31_task},
    config,
    tasks::data_manager::SpawnArg as DataManagerSpawnArg,
    tasks::sgp41::SpawnArg as Sgp41SpawnArg,
};
use log::debug;
use stm32f4xx_hal::prelude::*;

pub(crate) fn sht31_task(ctx: sht31_task::Context) {
    let sensor = &mut ctx.shared.i2c_devices.sht31;
    let (raw, measurement) = sensor.measure().unwrap();
    debug!("{measurement}");

    data_manager_task::spawn(DataManagerSpawnArg::Sht31Measurement(measurement)).unwrap();
    sgp41_task::spawn(Sgp41SpawnArg::ConditioningData(raw)).unwrap();
    sht31_task::spawn_after(config::SHT31_MEASUREMENT_INTERVAL_MS.millis()).unwrap();
}
