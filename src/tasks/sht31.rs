//use crate::app::{data_manager_task, sht31_task};
//use crate::tasks::SpawnArg;
use crate::{
    app::{sgp41_task, sht31_task},
    config,
    tasks::sgp41::SpawnArg as Sgp41SpawnArg,
};
use log::info;
use stm32f4xx_hal::prelude::*;

pub(crate) fn sht31_task(ctx: sht31_task::Context) {
    let sensor = &mut ctx.shared.i2c_devices.sht31;
    let (raw, measurement) = sensor.measure().unwrap();
    info!("{measurement}");

    // TODO
    //data_manager_task::spawn(SpawnArg::Sht31Measurement(measurement)).ok();

    sgp41_task::spawn(Sgp41SpawnArg::ConditioningData(raw)).unwrap();
    sht31_task::spawn_after(config::SHT31_MEASUREMENT_INTERVAL_MS.millis()).unwrap();
}
