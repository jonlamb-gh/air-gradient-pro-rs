use crate::{
    app::{data_manager_task, sgp41_task},
    config,
    sensors::sgp41::default_compensation,
    sensors::sht31,
    tasks::data_manager::SpawnArg as DataManagerSpawnArg,
};
use gas_index_algorithm::{AlgorithmType, GasIndexAlgorithm};
use log::{info, warn};
use stm32f4xx_hal::prelude::*;

/// Number of measurement update cycles to perform conditioning.
/// Run conditioning for the first 10 seconds (based on SGP41_MEASUREMENT_INTERVAL_MS).
const CONDITIONING_ITERS_10S: u32 = (10 * 1000) / config::SGP41_MEASUREMENT_INTERVAL_MS;

// Sample interval set to 1.0 seconds
// TODO const_assert_eq!(config::SGP41_MEASUREMENT_INTERVAL_MS, 1000)

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct GasIndices {
    pub voc_index: u16,
    pub nox_index: u16,
}

pub struct TaskState {
    // TODO - use a state machine enum to represent this, like done in pms task
    algos_init: bool,
    conditioning_iterations: u32,
    has_valid_compensation_data: bool,
    compensation_data: sht31::RawMeasurement,
    voc_algorithm: GasIndexAlgorithm,
    nox_algorithm: GasIndexAlgorithm,
}

impl TaskState {
    pub const fn new() -> Self {
        Self {
            algos_init: false,
            conditioning_iterations: 0,
            has_valid_compensation_data: false,
            compensation_data: default_compensation(),
            voc_algorithm: GasIndexAlgorithm::new_uninitialized(AlgorithmType::Voc),
            nox_algorithm: GasIndexAlgorithm::new_uninitialized(AlgorithmType::Nox),
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

    if !state.algos_init {
        state.algos_init = true;
        state.voc_algorithm.init_with_sampling_interval(1.0);
        state.nox_algorithm.init_with_sampling_interval(1.0);
    }

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
                let gas_indices = GasIndices {
                    voc_index: state.voc_algorithm.process(measurement.voc_ticks as _) as _,
                    nox_index: state.nox_algorithm.process(measurement.nox_ticks as _) as _,
                };

                info!("{measurement}");

                data_manager_task::spawn(DataManagerSpawnArg::Sgp41Measurement(measurement))
                    .unwrap();
                data_manager_task::spawn(DataManagerSpawnArg::GasIndices(gas_indices)).unwrap();
            }

            sgp41_task::spawn_after(
                config::SGP41_MEASUREMENT_INTERVAL_MS.millis(),
                SpawnArg::Measurement,
            )
            .unwrap();
        }
    }
}
