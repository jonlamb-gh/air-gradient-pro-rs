use crate::sensors::sht31;
use crate::{app::sgp41_task, config, sensors::sgp41::default_compensation, sensors::Sgp41};
//use crate::tasks::SpawnArg;
//use crate::app::{data_manager_task, sgp41_task};
use log::{info, warn};
use stm32f4xx_hal::prelude::*;

/// Number of measurement update cycles to perform conditioning.
/// Run conditioning for the first 10 seconds (based on SGP41_MEASUREMENT_INTERVAL_MS).
const CONDITIONING_ITERS_10S: u32 = (10 * 1000) / config::SGP41_MEASUREMENT_INTERVAL_MS;

pub struct TaskState {
    conditioning_iterations: u32,
    has_valid_compensation_data: bool,
    compensation_data: sht31::RawMeasurement,
}

impl TaskState {
    pub const fn new() -> Self {
        Self {
            conditioning_iterations: 0,
            has_valid_compensation_data: false,
            compensation_data: default_compensation(),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum SpawnArg {
    /// Raw temperature and humidity measurement from the SHT31 sensor
    /// used for compensation.
    ConditioningData(sht31::RawMeasurement),
    /// Periodic measurement interval
    Measurement,
}

// TODO - needs a spawn arg for sht31 vs measurement update
pub(crate) fn sgp41_task(ctx: sgp41_task::Context, arg: SpawnArg) {
    // TODO
    // do the voc/nox algorithm processing here for converting
    // raw signals to index values
    //
    // do conditioning for the first 10 seconds, then do raw signal measurements
    // requires temp/humid data from sht31
    let state = ctx.local.state;
    let sensor = &mut ctx.shared.i2c_devices.sgp41;

    match arg {
        SpawnArg::ConditioningData(cond_data) => {
            if !state.has_valid_compensation_data {
                info!("SGP41: received compensation data");
            }
            state.compensation_data = cond_data;
            state.has_valid_compensation_data = true;
        }
        SpawnArg::Measurement => {
            if state.conditioning_iterations < CONDITIONING_ITERS_10S {
                if state.conditioning_iterations == 0 {
                    info!("SGP41: start conditioning");
                }
                sensor.execute_conditioning().unwrap();
                state.conditioning_iterations += 1;
                if state.conditioning_iterations == CONDITIONING_ITERS_10S {
                    info!("SGP41: conditioning complete, starting measurements");
                }
            } else {
                if !state.has_valid_compensation_data {
                    warn!("SGP41: no compensation data, using default");
                }
                let measurement = sensor.measure(&state.compensation_data).unwrap();
                info!("{measurement}");

                // TODO
                //data_manager_task::spawn(SpawnArg::Sgp41Measurement(measurement)).ok();
            }

            sgp41_task::spawn_after(
                config::SGP41_MEASUREMENT_INTERVAL_MS.millis(),
                SpawnArg::Measurement,
            )
            .unwrap();
        }
    }
}
