use crate::firmware_main::app::sht31_task;
use crate::sensors::Sht31;
use log::info;
use stm32f4xx_hal::prelude::*;

pub(crate) fn sht31_task(ctx: sht31_task::Context) {
    // TODO
    let sensor = &mut ctx.shared.i2c_devices.sht31;
    info!("Meausre SHT31");
    let measurement = sensor.measure().unwrap();
    info!("{measurement:#?}");
    sht31_task::spawn_after(Sht31::<(), ()>::MEASUREMENT_PERIOD_MS.millis()).ok();
}
