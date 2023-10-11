#![no_std]
#![forbid(unsafe_code)]

use bootloader_support::{BootSlot, ResetReason};
use core::fmt::{self, Write};
use log::{debug, warn};
use smoltcp::socket::tcp::{self, Socket as TcpSocket};
use wire_protocols::{
    device::{
        Command, MemoryEraseRequest, MemoryReadRequest, MemoryRegion, MemoryWriteRequest,
        StatusCode,
    },
    DeviceId, DeviceSerialNumber, FirmwareVersion, ProtocolVersion,
};

pub trait Device {
    fn info(&self) -> &DeviceInfo;
    fn perform_reboot(&mut self) -> !;
    fn complete_update_and_perform_reboot(&mut self) -> !;
    // TODO
    // StatusCode has Success... use a different error type
    fn read_memory(&mut self, req: MemoryReadRequest) -> StatusCodeResult<&[u8]>;
    fn write_memory(&mut self, req: MemoryWriteRequest, data: &[u8]) -> StatusCodeResult<()>;
    fn erase_memory(&mut self, req: MemoryEraseRequest) -> StatusCodeResult<()>;
}

pub type StatusCodeResult<T> = core::result::Result<T, StatusCode>;
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    Connect(tcp::ConnectError),
    Listen(tcp::ListenError),
    Recv(tcp::RecvError),
    Send(tcp::SendError),
    Fmt,
    Protocol,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DeviceInfo {
    pub protocol_version: ProtocolVersion,
    pub firmware_version: FirmwareVersion,
    pub device_id: DeviceId,
    pub device_serial_number: DeviceSerialNumber,
    pub mac_address: [u8; 6],
    pub active_boot_slot: BootSlot,
    pub reset_reason: ResetReason,
    pub built_time_utc: &'static str,
    pub git_commit: &'static str,
}

pub const UPDATE_TICKS_TO_REBOOT: usize = 10;
pub const UPDATE_TICKS_TO_CLOSE: usize = UPDATE_TICKS_TO_REBOOT / 2;

type RemainingMemoryWriteRegion = MemoryRegion;

pub struct UpdateManager {
    port: u16,
    update_complete: bool,
    update_in_progress: bool,
    write_in_progress: Option<RemainingMemoryWriteRegion>,
    ticks_until_reboot: Option<usize>,
}

impl UpdateManager {
    pub const fn new(port: u16) -> Self {
        Self {
            port,
            update_complete: false,
            update_in_progress: false,
            write_in_progress: None,
            ticks_until_reboot: None,
        }
    }

    pub fn reset(&mut self, socket: &mut TcpSocket) {
        self.abort_in_progress(socket);
    }

    pub fn update<D: Device>(&mut self, device: &mut D, socket: &mut TcpSocket) -> Result<()> {
        self.manage_reboot_schedule(device, socket);

        self.manage_socket(socket)?;

        if let Some(remaining_region) = self.write_in_progress.take() {
            self.manage_in_progress_write(remaining_region, device, socket)?;
        } else if let Some(cmd) = self.recv_cmd(socket)? {
            self.process_cmd(cmd, device, socket)?;
        }

        Ok(())
    }

    fn manage_reboot_schedule<D: Device>(&mut self, device: &mut D, socket: &mut TcpSocket) {
        if let Some(ticks_until_reboot) = self.ticks_until_reboot.as_mut() {
            *ticks_until_reboot = ticks_until_reboot.saturating_sub(1);
            if *ticks_until_reboot == UPDATE_TICKS_TO_CLOSE {
                debug!("Closing connection ahead of reboot");
                socket.close();
            } else if *ticks_until_reboot == 0 {
                debug!("Time to reboot");

                if self.update_complete {
                    device.complete_update_and_perform_reboot();
                } else {
                    device.perform_reboot();
                }
            }
        }
    }

    fn abort_in_progress(&mut self, socket: &mut TcpSocket) {
        if self.write_in_progress.is_some() {
            warn!("In-progress write will be aborted");
        }
        if self.update_in_progress {
            warn!("In-progress update will be aborted");
        }
        debug!(
            "Aborting socket, send_queue {} ({}), recv_queue {} ({})",
            socket.send_queue(),
            socket.send_capacity(),
            socket.recv_queue(),
            socket.recv_capacity(),
        );
        self.update_in_progress = false;
        self.write_in_progress = None;
        self.update_complete = false;
        socket.abort();
    }

    fn manage_socket(&mut self, socket: &mut TcpSocket) -> Result<()> {
        if !socket.is_open() {
            debug!("UM: listening on port {}", self.port);
            socket.listen(self.port)?;
        }

        if !socket.may_recv() && socket.may_send() {
            debug!("UM: closing socket due to lack of recv");
            self.abort_in_progress(socket);
        }

        Ok(())
    }

    fn send_status(&mut self, status: StatusCode, socket: &mut TcpSocket) -> Result<()> {
        if !socket.can_send() {
            warn!("Cannot send status {status}, aborting");
            self.abort_in_progress(socket);
        } else {
            let bytes = u32::from(status).to_le_bytes();
            socket.send_slice(&bytes)?;
        }
        Ok(())
    }

    fn manage_in_progress_write<D: Device>(
        &mut self,
        remaining_region: MemoryRegion,
        device: &mut D,
        socket: &mut TcpSocket,
    ) -> Result<()> {
        self.handle_write_req_data(remaining_region, device, socket)?;
        Ok(())
    }

    fn recv_cmd(&mut self, socket: &mut TcpSocket) -> Result<Option<Command>> {
        if socket.can_recv() && socket.recv_queue() >= 4 {
            let mut buf = [0_u8; 4];
            let bytes_recvd = socket.recv_slice(&mut buf)?;
            if bytes_recvd < 4 {
                warn!("Invalid command bytes recvd, aborting");
                self.send_status(StatusCode::CommandLengthIncorrect, socket)?;
                self.abort_in_progress(socket);
                Ok(None)
            } else {
                Ok(Some(Command::from_le_bytes_unchecked(&buf)))
            }
        } else {
            Ok(None)
        }
    }

    fn process_cmd<D: Device>(
        &mut self,
        cmd: Command,
        device: &mut D,
        socket: &mut TcpSocket,
    ) -> Result<()> {
        debug!("UM: processing command {cmd}");
        match cmd {
            Command::Info => {
                let dev_info = device.info();
                self.send_status(StatusCode::Success, socket)?;
                writeln!(socket, "{{\"protocol_version\": \"{}\", \"firmware_version\": \"{}\", \"device_id\": {}, \"device_serial_number\": \"{:X}\", \"mac_address\": {:?}, \"active_boot_slot\": \"{}\", \"reset_reason\": \"{}\", \"built_time_utc\": \"{}\", \"git_commit\": \"{}\"}}",
                    dev_info.protocol_version,
                    dev_info.firmware_version,
                    dev_info.device_id,
                    dev_info.device_serial_number,
                    dev_info.mac_address,
                    dev_info.active_boot_slot,
                    dev_info.reset_reason,
                    dev_info.built_time_utc,
                    dev_info.git_commit,
                )?;

                socket.close();

                if self.update_in_progress || self.write_in_progress.is_some() {
                    self.abort_in_progress(socket);
                }
            }
            Command::ReadMemory => {
                let mem_region = self.read_mem_region(socket)?;
                debug!(
                    "Read region address=0x{:X}, len=0x{:X}",
                    mem_region.address, mem_region.length
                );

                match device.read_memory(mem_region) {
                    Ok(region) => {
                        self.send_status(StatusCode::Success, socket)?;
                        socket.send_slice(region)?;
                    }
                    Err(code) => {
                        warn!("Device returned status {code}");
                        self.send_status(code, socket)?
                    }
                }
            }
            Command::WriteMemory => {
                let mem_region = self.read_mem_region(socket)?;
                debug!(
                    "Write region address=0x{:X}, len=0x{:X}",
                    mem_region.address, mem_region.length
                );

                self.handle_write_req_data(mem_region, device, socket)?;
            }
            Command::EraseMemory => {
                let mem_region = self.read_mem_region(socket)?;
                debug!(
                    "Erase region address=0x{:X}, len=0x{:X}",
                    mem_region.address, mem_region.length
                );

                match device.erase_memory(mem_region) {
                    Ok(()) => {
                        self.send_status(StatusCode::Success, socket)?;
                    }
                    Err(code) => {
                        warn!("Device returned status {code}");
                        self.send_status(code, socket)?
                    }
                }
            }
            Command::CompleteAndReboot => {
                debug!(
                    "UM: scheduling a reobot {} update cycles from now",
                    UPDATE_TICKS_TO_REBOOT
                );

                // TODO - update the protocol to indicate this
                self.update_complete = true;

                self.send_status(StatusCode::Success, socket)?;
                self.ticks_until_reboot = Some(UPDATE_TICKS_TO_REBOOT);
            }
            Command::Unknown(_c) => {
                self.send_status(StatusCode::UnknownCommand, socket)?;
            }
        }
        Ok(())
    }

    fn read_mem_region(&mut self, socket: &mut TcpSocket) -> Result<MemoryRegion> {
        let mut addr = [0_u8; 4];
        let mut len = [0_u8; 4];
        let addr_bytes_recvd = socket.recv_slice(&mut addr);
        let len_bytes_recvd = socket.recv_slice(&mut len);
        if addr_bytes_recvd.is_err() || len_bytes_recvd.is_err() {
            self.send_status(StatusCode::NetworkError, socket)?;
            let _ = addr_bytes_recvd?;
            let _ = len_bytes_recvd?;
            Err(Error::Protocol)
        } else if addr_bytes_recvd.unwrap_or(0) != 4 || len_bytes_recvd.unwrap_or(0) != 4 {
            self.send_status(StatusCode::CommandLengthIncorrect, socket)?;
            Err(Error::Protocol)
        } else {
            Ok(MemoryRegion {
                address: u32::from_le_bytes(addr),
                length: u32::from_le_bytes(len),
            })
        }
    }

    // TODO - check for 16-byte (128 bit) alignment?
    fn handle_write_req_data<D: Device>(
        &mut self,
        mem_region: MemoryRegion,
        device: &mut D,
        socket: &mut TcpSocket,
    ) -> Result<()> {
        let mut recv_handler = |buf: &[u8]| {
            let region_size = mem_region.length as usize;
            if buf.len() >= region_size {
                // Can fulfil the entire write
                (
                    region_size,
                    device.write_memory(mem_region, &buf[..region_size]),
                )
            } else {
                // Write what's available, read the rest as it comes in
                // TODO may need to enforce 4-byte len
                debug!("Partial write of {} (total {})", buf.len(), region_size);
                let partial_region_to_write = MemoryRegion {
                    address: mem_region.address,
                    length: buf.len() as u32,
                };
                let partial_region_remaining = MemoryRegion {
                    address: mem_region.address + partial_region_to_write.length,
                    length: mem_region.length - partial_region_to_write.length,
                };

                self.write_in_progress = Some(partial_region_remaining);

                (buf.len(), device.write_memory(partial_region_to_write, buf))
            }
        };

        match socket.recv(|buf| recv_handler(buf)) {
            Ok(Ok(())) => {
                self.update_in_progress = true;

                // Don't send status until the write request is fulfilled
                if self.write_in_progress.is_none() {
                    self.send_status(StatusCode::Success, socket)?;
                }
            }
            Ok(Err(code)) => {
                warn!("Device returned status {code}");
                self.send_status(code, socket)?;
                self.abort_in_progress(socket);
            }
            Err(_) => {
                self.send_status(StatusCode::NetworkError, socket)?;
                self.abort_in_progress(socket);
            }
        }

        Ok(())
    }
}

impl From<tcp::ConnectError> for Error {
    fn from(value: tcp::ConnectError) -> Self {
        Error::Connect(value)
    }
}

impl From<tcp::ListenError> for Error {
    fn from(value: tcp::ListenError) -> Self {
        Error::Listen(value)
    }
}

impl From<tcp::RecvError> for Error {
    fn from(value: tcp::RecvError) -> Self {
        Error::Recv(value)
    }
}

impl From<tcp::SendError> for Error {
    fn from(value: tcp::SendError) -> Self {
        Error::Send(value)
    }
}

impl From<fmt::Error> for Error {
    fn from(_value: fmt::Error) -> Self {
        Error::Fmt
    }
}
