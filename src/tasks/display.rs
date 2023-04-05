use crate::{
    app::display_task,
    display::{SystemInfo, SystemStatus},
    util,
};
use log::info;

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum SpawnArg {
    Startup,
    SystemStatus(SystemStatus),
}

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

pub(crate) fn display_task(ctx: display_task::Context, arg: SpawnArg) {
    let state = ctx.local.state;
    let display = &mut ctx.shared.i2c_devices.display;

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
