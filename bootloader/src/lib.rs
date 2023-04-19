#![no_std]
// TODO lints/etc

mod boot_config;
mod reset_reason;
mod ucs;

pub use crate::boot_config::{BootConfig, BootSlotExt, DEFAULT_CONFIG};
pub use crate::reset_reason::ResetReason;
pub use crate::ucs::UpdateConfigAndStatus;

/// Initiate a system reset request to reset the MCU
///
/// # Safety
/// This is a reboot.
pub unsafe fn sw_reset() -> ! {
    cortex_m::peripheral::SCB::sys_reset();
}
