use crate::firmware_main::app::{data_manager_task, sgp41_task};
use crate::sensors::Sgp41;
use crate::tasks::SpawnArg;
use log::info;
use stm32f4xx_hal::prelude::*;

pub(crate) fn sgp41_task(ctx: sgp41_task::Context) {
    // TODO
    let sensor = &mut ctx.shared.i2c_devices.sgp41;
    //info!("Meausre SHT31");
    let measurement = sensor.measure().unwrap();
    info!("{measurement}");

    data_manager_task::spawn(SpawnArg::Sgp41Measurement(measurement)).ok();

    // TODO - wrapper tasks will reschedule
    sgp41_task::spawn_after(Sgp41::<(), ()>::MEASUREMENT_PERIOD_MS.millis()).ok();
}
