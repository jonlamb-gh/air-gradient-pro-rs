use crate::{
    app::{data_manager_task, sgp41_task},
    config,
    sensors::sgp41::default_compensation,
    sensors::sht31,
    tasks::data_manager::SpawnArg as DataManagerSpawnArg,
};
use core::num::NonZeroU16;
use gas_index_algorithm::{AlgorithmType, GasIndexAlgorithm};
use log::{debug, warn};
use static_assertions::const_assert_eq;
use stm32f4xx_hal::prelude::*;

// SGP41 task requires 1 second cycles
const_assert_eq!(config::SGP41_MEASUREMENT_INTERVAL_MS, 1000);

/// Number of measurement update cycles to perform conditioning.
/// Run conditioning for the first 10 seconds (based on SGP41_MEASUREMENT_INTERVAL_MS).
const CONDITIONING_ITERS_10S: u32 = (10 * 1000) / config::SGP41_MEASUREMENT_INTERVAL_MS;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct GasIndices {
    /// Calculated VOC gas index value.
    /// Zero during initial blackout period and 1..500 afterwards.
    pub voc_index: Option<NonZeroU16>,

    /// Calculated VOC gas index value.
    /// Zero during initial blackout period and 1..500 afterwards.
    pub nox_index: Option<NonZeroU16>,
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

pub(crate) fn sgp41_task(ctx: sgp41_task::Context, arg: SpawnArg) {
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
                debug!("SGP41: received compensation data");
            }
            state.compensation_data = cond_data;
            state.has_valid_compensation_data = true;
        }
        SpawnArg::Measurement => {
            if state.conditioning_iterations < CONDITIONING_ITERS_10S {
                if state.conditioning_iterations == 0 {
                    debug!("SGP41: start conditioning");
                }
                sensor.execute_conditioning().unwrap();
                state.conditioning_iterations += 1;
                if state.conditioning_iterations == CONDITIONING_ITERS_10S {
                    debug!("SGP41: conditioning complete, starting measurements");
                }
            } else {
                if !state.has_valid_compensation_data {
                    warn!("SGP41: no compensation data, using default");
                }
                let measurement = sensor.measure(&state.compensation_data).unwrap();
                let gas_indices = GasIndices {
                    voc_index: NonZeroU16::new(
                        state.voc_algorithm.process(measurement.voc_ticks as _) as u16,
                    ),
                    nox_index: NonZeroU16::new(
                        state.nox_algorithm.process(measurement.nox_ticks as _) as u16,
                    ),
                };

                debug!("{measurement}");

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
