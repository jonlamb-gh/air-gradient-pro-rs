use crate::{app::watchdog_task, config};
use stm32f4xx_hal::prelude::*;

pub(crate) fn watchdog_task(ctx: watchdog_task::Context) {
    let watchdog = ctx.local.watchdog;
    let led = ctx.local.led;

    watchdog.feed();
    led.toggle();

    watchdog_task::spawn_after(config::WATCHDOG_TASK_INTERVAL_MS.millis()).unwrap();
}
