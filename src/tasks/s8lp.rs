//use crate::app::{data_manager_task, s8lp_task};
//use crate::tasks::SpawnArg;
use crate::{app::s8lp_task, config};
use log::info;
use stm32f4xx_hal::prelude::*;

pub(crate) fn s8lp_task(ctx: s8lp_task::Context) {
    let sensor = ctx.local.s8lp;
    let measurement = sensor.measure().unwrap();
    info!("{measurement}");

    // TODO
    //data_manager_task::spawn(SpawnArg::S8LpMeasurement(measurement)).ok();

    s8lp_task::spawn_after(config::S8LP_MEASUREMENT_INTERVAL_MS.millis()).unwrap();
}
