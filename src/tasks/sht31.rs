use crate::app::{data_manager_task, sht31_task};
use crate::sensors::Sht31;
use crate::tasks::SpawnArg;
use log::info;
use stm32f4xx_hal::prelude::*;

pub(crate) fn sht31_task(ctx: sht31_task::Context) {
    // TODO
    let sensor = &mut ctx.shared.i2c_devices.sht31;
    //info!("Meausre SHT31");
    let measurement = sensor.measure().unwrap();
    info!("{measurement}");

    data_manager_task::spawn(SpawnArg::Sht31Measurement(measurement)).ok();
}
