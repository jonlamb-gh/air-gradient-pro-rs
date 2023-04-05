use crate::{
    app::display_task,
    display::{SystemInfo, SystemStatus},
    util,
};
use log::info;

pub struct TaskState {
    sys_info: SystemInfo,
    sys_status: SystemStatus,
}

impl TaskState {
    pub const fn new() -> Self {
        Self {
            sys_info: SystemInfo::new(),
            sys_status: SystemStatus::new(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum SpawnArg {
    Startup,
    SystemStatus(SystemStatus),
}

pub(crate) fn display_task(ctx: display_task::Context, arg: SpawnArg) {
    let state = ctx.local.state;
    let display = &mut ctx.shared.i2c_devices.display;

    // On startup (the initial spawn) we draw system info once
    // and it stays rendered until ....
    // TODO until conditioning done on sensor or until first bcast tx?
    // clear screen between state/view changes

    match arg {
        SpawnArg::Startup => {
            if state.sys_info.device_serial_number.is_zero() {
                info!("Initializing display state");
                state.sys_info.device_serial_number = util::read_device_serial_number();
                display.render_system_info(&state.sys_info).unwrap();
            }
        }
        SpawnArg::SystemStatus(status) => {
            state.sys_status = status;
            display.render_system_status(&state.sys_status).unwrap();
        }
    }
}
