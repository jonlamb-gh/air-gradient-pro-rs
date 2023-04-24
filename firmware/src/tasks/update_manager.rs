use crate::{app::update_manager_task, config};
use bootloader_lib::UpdateConfigAndStatus;
use bootloader_support::FLASH_BASE_ADDRESS;
use log::{debug, warn};
use smoltcp::socket::tcp::Socket as TcpSocket;
use stm32f4xx_hal::{
    flash::FlashExt,
    pac::{self, FLASH},
    prelude::*,
    rcc::Enable,
};
use update_manager::{Device, DeviceInfo, StatusCodeResult, UpdateManager};
use wire_protocols::device::{
    MemoryEraseRequest, MemoryReadRequest, MemoryWriteRequest, StatusCode,
};

pub struct TaskState {
    um: UpdateManager,
}

impl TaskState {
    pub const fn new() -> Self {
        Self {
            um: UpdateManager::new(config::DEVICE_PORT),
        }
    }
}

pub(crate) fn update_manager_task(ctx: update_manager_task::Context) {
    let state = ctx.local.state;
    let device_info = ctx.local.device_info;
    let flash = ctx.local.flash;
    let sockets = ctx.shared.sockets;
    let socket_handle = ctx.shared.device_socket;

    // TODO
    // socket keep_alive and timeout configs
    let socket = sockets.get_mut::<TcpSocket>(*socket_handle);

    let mut dev = UmDevice {
        info: device_info,
        flash,
    };
    if let Err(e) = state.um.update(&mut dev, socket) {
        warn!("UM: returned an error. {e:?}");
        state.um.reset(socket);
    }

    update_manager_task::spawn_after(config::UPDATE_MANAGER_POLL_INTERVAL_MS.millis()).unwrap();
}

struct UmDevice<'a> {
    info: &'a DeviceInfo,
    flash: &'a mut FLASH,
}

impl<'a> Device for UmDevice<'a> {
    fn info(&self) -> &DeviceInfo {
        self.info
    }

    fn perform_reboot(&mut self) -> ! {
        warn!("Rebooting now");
        unsafe {
            // TODO - this is common in several places, put into a fn (see main.rs)
            crate::logger::flush_logger();
            let rcc = &(*pac::RCC::ptr());
            pac::USART6::disable(rcc);

            bootloader_lib::sw_reset();
        }
    }

    fn complete_update_and_perform_reboot(&mut self) -> ! {
        warn!("Update complete, rebooting now");
        UpdateConfigAndStatus::set_update_pending();
        unsafe {
            // TODO - this is common in several places, put into a fn (see main.rs)
            crate::logger::flush_logger();
            let rcc = &(*pac::RCC::ptr());
            pac::USART6::disable(rcc);

            bootloader_lib::sw_reset();
        }
    }

    fn read_memory(&mut self, req: MemoryReadRequest) -> StatusCodeResult<&[u8]> {
        let other_slot = self.info.active_boot_slot.other();
        if !other_slot.contains(req.address) {
            Err(StatusCode::InvalidAddress)
        } else if !other_slot.contains(req.address + req.length - 1) {
            Err(StatusCode::DataLengthIncorrect)
        } else {
            req.check_length()?;
            let a = (req.address - FLASH_BASE_ADDRESS) as usize;
            let b = a + (req.length as usize);
            debug!("Reading FLASH at offset 0x{a:X} len=0x{:X}", req.length);
            let mem = self.flash.read();
            Ok(&mem[a..b])
        }
    }

    fn write_memory(&mut self, req: MemoryWriteRequest, data: &[u8]) -> StatusCodeResult<()> {
        let other_slot = self.info.active_boot_slot.other();
        if !other_slot.contains(req.address) {
            Err(StatusCode::InvalidAddress)
        } else if !other_slot.contains(req.address + req.length - 1) {
            Err(StatusCode::DataLengthIncorrect)
        } else {
            req.check_length()?;
            let offset = req.address - FLASH_BASE_ADDRESS;
            debug!(
                "Writing to FLASH at offset 0x{offset:X} len=0x{:X}",
                req.length
            );
            let mut unlocked_flash = self.flash.unlocked();
            unlocked_flash
                .program(offset as usize, data.iter())
                .map_err(|e| {
                    warn!("Flash write error: {e:?}");
                    StatusCode::WriteError
                })?;
            Ok(())
        }
    }

    fn erase_memory(&mut self, req: MemoryEraseRequest) -> StatusCodeResult<()> {
        let other_slot = self.info.active_boot_slot.other();
        if req.address != other_slot.address() {
            Err(StatusCode::InvalidAddress)
        } else if req.length != other_slot.size() {
            Err(StatusCode::DataLengthIncorrect)
        } else {
            let mut unlocked_flash = self.flash.unlocked();
            for sector in other_slot.sectors() {
                unlocked_flash.erase(*sector).map_err(|e| {
                    warn!("Flash sector {sector} erase error: {e:?}");
                    StatusCode::EraseError
                })?;
            }
            Ok(())
        }
    }
}
