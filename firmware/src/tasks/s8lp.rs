use crate::{
    app::{data_manager_task, s8lp_task},
    config,
    tasks::data_manager::SpawnArg as DataManagerSpawnArg,
};
use log::debug;
use stm32f4xx_hal::prelude::*;

pub(crate) fn s8lp_task(ctx: s8lp_task::Context) {
    let sensor = ctx.local.s8lp;
    let measurement = sensor.measure().unwrap();
    debug!("{measurement}");

    data_manager_task::spawn(DataManagerSpawnArg::S8LpMeasurement(measurement)).unwrap();
    s8lp_task::spawn_after(config::S8LP_MEASUREMENT_INTERVAL_MS.millis()).unwrap();
}
