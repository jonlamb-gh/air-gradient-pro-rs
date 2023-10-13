use crate::{
    app::display_task,
    display::{FirmwareUpdateInfo, SystemInfo, SystemStatus},
    util,
};
use log::debug;

const DEFAULT_IGNORE: usize = 6;

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum SpawnArg {
    Startup,
    SystemStatus(SystemStatus),
    FirmwareUpdateInfo(FirmwareUpdateInfo),
}

pub struct TaskState {
    sys_info: SystemInfo,
    sys_status: SystemStatus,
    /// Number of non-fw-update requests to ignore while fw update
    /// is in-progress
    requests_to_ignore_while_updating: usize,
}

impl TaskState {
    pub const fn new() -> Self {
        Self {
            sys_info: SystemInfo::new(),
            sys_status: SystemStatus::new(),
            requests_to_ignore_while_updating: 0,
        }
    }
}

pub(crate) fn display_task(ctx: display_task::Context, arg: SpawnArg) {
    let state = ctx.local.state;
    let display = &mut ctx.shared.i2c_devices.display;

    state.requests_to_ignore_while_updating =
        state.requests_to_ignore_while_updating.saturating_sub(1);

    match arg {
        SpawnArg::Startup => {
            if state.sys_info.device_serial_number.is_zero() {
                debug!("Initializing display state");
                state.sys_info.device_serial_number = util::read_device_serial_number();
                display.render_system_info(&state.sys_info).unwrap();
            }
        }
        SpawnArg::SystemStatus(status) => {
            state.sys_status = status;
            if state.requests_to_ignore_while_updating == 0 {
                display.render_system_status(&state.sys_status).unwrap();
            }
        }
        SpawnArg::FirmwareUpdateInfo(info) => {
            state.requests_to_ignore_while_updating = DEFAULT_IGNORE;
            display.render_firmware_update_info(&info).unwrap();
        }
    }
}
