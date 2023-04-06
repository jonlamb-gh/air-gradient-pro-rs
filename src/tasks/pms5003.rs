use crate::{
    app::{data_manager_task, pms5003_task},
    config,
    tasks::data_manager::SpawnArg as DataManagerSpawnArg,
};
use static_assertions::{const_assert, const_assert_eq};
use stm32f4xx_hal::prelude::*;

// This task requires 1 second cycles
const_assert_eq!(config::PMS5003_MEASUREMENT_INTERVAL_MS, 1000);

// At least 30 seconds required to warm up
const_assert!(config::PMS5003_WARM_UP_PERIOD_MS >= 30_000);

// Should be in standby for longer that the warmup
const_assert!(config::PMS5003_WAKE_INTERVAL_MS > config::PMS5003_WARM_UP_PERIOD_MS);

const WAKE_INTERVAL_TICKS: u32 =
    config::PMS5003_WAKE_INTERVAL_MS / config::PMS5003_MEASUREMENT_INTERVAL_MS;
const WARM_UP_PERIOD_TICKS: u32 =
    config::PMS5003_WARM_UP_PERIOD_MS / config::PMS5003_MEASUREMENT_INTERVAL_MS;

pub struct TaskState {
    state: State,
}

type TicksUntilWakeUp = u32;
type TicksUntilMeasurement = u32;
type MeasurementsUntilStandby = u8;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum State {
    /// The sensor is in standby mode.
    /// Starts at WAKE_INTERVAL_TICKS, decrements until zero, then
    /// the sensor enters active mode to warm up.
    StandbyMode(TicksUntilWakeUp),

    /// The sensor is in active mode and warming up.
    /// Once woken up, starts at WARM_UP_PERIOD_TICKS, decrements until
    /// zero, then the tasks starts requesting measurements from the
    /// sensor.
    WarmingUp(TicksUntilMeasurement),

    /// The sensor is in active mode and requesting a measurement
    /// each task iteration. Once warmed up, starts at PMS5003_MEASUREMENT_COUNT,
    /// decrements until zero, then the sensor goes back into standby mode.
    Measuring(MeasurementsUntilStandby),
}

impl State {
    /// Starts at the end of the StandbyMode state, will start
    /// warming up after initialized.
    const fn begin_warm_up() -> Self {
        State::StandbyMode(0)
    }

    const fn init() -> Self {
        State::StandbyMode(WAKE_INTERVAL_TICKS)
    }
}

impl TaskState {
    pub const fn new() -> Self {
        Self {
            state: State::begin_warm_up(),
        }
    }
}

pub(crate) fn pms5003_task(ctx: pms5003_task::Context) {
    let state = &mut ctx.local.state.state;
    let sensor = ctx.local.pms;
    // TODO improve this state machine methods on State, tick/update
    match state {
        State::StandbyMode(ticks_until_wake_up) => {
            *ticks_until_wake_up = ticks_until_wake_up.saturating_sub(1);
            if *ticks_until_wake_up == 0 {
                log::debug!("PMS5003: entering active mode");
                sensor.enter_active_mode().unwrap();
                *state = State::WarmingUp(WARM_UP_PERIOD_TICKS);
            }
        }
        State::WarmingUp(ticks_until_measurement) => {
            *ticks_until_measurement = ticks_until_measurement.saturating_sub(1);
            if *ticks_until_measurement == 0 {
                log::debug!("PMS5003: begin measuring");
                *state = State::Measuring(config::PMS5003_MEASUREMENT_COUNT);
            }
        }
        State::Measuring(measurements_until_standby) => {
            let measurement = sensor.measure().unwrap();
            log::debug!("{measurement}");

            *measurements_until_standby = measurements_until_standby.saturating_sub(1);
            if *measurements_until_standby == 0 {
                log::debug!("PMS5003: entering standby mode");
                sensor.enter_standby_mode().unwrap();
                *state = State::init();
            }

            data_manager_task::spawn(DataManagerSpawnArg::Pms5003Measurement(measurement)).unwrap();
        }
    }

    pms5003_task::spawn_after(config::PMS5003_MEASUREMENT_INTERVAL_MS.millis()).unwrap();
}
