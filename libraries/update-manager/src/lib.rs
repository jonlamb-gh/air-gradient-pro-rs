#![no_std]
#![forbid(unsafe_code)]

use bootloader_support::{BootSlot, ResetReason};
use core::fmt::{self, Write};
use log::{debug, warn};
use smoltcp::socket::tcp::{self, Socket as TcpSocket};
use wire_protocols::{
    device::{Command, StatusCode},
    DeviceId, DeviceSerialNumber, FirmwareVersion, ProtocolVersion,
};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    Connect(tcp::ConnectError),
    Listen(tcp::ListenError),
    Recv(tcp::RecvError),
    Send(tcp::SendError),
    Fmt,
}

// TODO
//pub enum ActionToTake or State/Status? { None, ScheduleReboot , UpdatePending{}...
//  MemoryErase(MemoryEraseRequest)...
//
// caller deals with flash work, or maybe take a closure or something
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ActionToTake {
    Reboot,
    CompleteAndReboot,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DeviceInfo {
    pub protocol_version: ProtocolVersion,
    pub firmware_version: FirmwareVersion,
    pub device_id: DeviceId,
    pub device_serial_number: DeviceSerialNumber,
    pub active_boot_slot: BootSlot,
    pub reset_reason: ResetReason,
    pub built_time_utc: &'static str,
    pub git_commit: &'static str,
}

pub const UPDATE_TICKS_TO_REBOOT: usize = 10;
pub const UPDATE_TICKS_TO_CLOSE: usize = UPDATE_TICKS_TO_REBOOT / 2;

pub struct UpdateManager {
    port: u16,
    update_in_progress: bool,
    ticks_until_reboot: Option<usize>,
}

impl UpdateManager {
    pub const fn new(port: u16) -> Self {
        Self {
            port,
            update_in_progress: false,
            ticks_until_reboot: None,
        }
    }

    pub fn update(
        &mut self,
        dev_info: &DeviceInfo,
        socket: &mut TcpSocket,
    ) -> Result<Option<ActionToTake>> {
        if let Some(action) = self.manage_reboot_schedule(socket) {
            return Ok(Some(action));
        }

        self.manage_socket(socket)?;

        if let Some(cmd) = self.recv_cmd(socket)? {
            debug!("UM: recvd command {cmd}");
            self.process_cmd(cmd, dev_info, socket)?;
        }

        Ok(None)
    }

    fn manage_reboot_schedule(&mut self, socket: &mut TcpSocket) -> Option<ActionToTake> {
        if let Some(ticks_until_reboot) = self.ticks_until_reboot.as_mut() {
            *ticks_until_reboot = ticks_until_reboot.saturating_sub(1);
            if *ticks_until_reboot == UPDATE_TICKS_TO_CLOSE {
                socket.close();
            } else if *ticks_until_reboot == 0 {
                debug!("Time to reboot");

                if self.update_in_progress {
                    return Some(ActionToTake::CompleteAndReboot);
                } else {
                    return Some(ActionToTake::Reboot);
                }
            }
        }
        None
    }

    fn abort_in_progress(&mut self, socket: &mut TcpSocket) {
        if self.update_in_progress {
            warn!("In-progress update will be aborted");
        }
        self.update_in_progress = false;
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

    fn process_cmd(
        &mut self,
        cmd: Command,
        dev_info: &DeviceInfo,
        socket: &mut TcpSocket,
    ) -> Result<()> {
        match cmd {
            Command::Info => {
                self.send_status(StatusCode::Success, socket)?;
                writeln!(socket, "{{\"protocol_version\": \"{}\", \"firmware_version\": \"{}\", \"device_id\": {}, \"device_serial_number\": \"{:X}\", \"active_boot_slot\": \"{}\", \"reset_reason\": \"{}\", \"built_time_utc\": \"{}\", \"git_commit\": \"{}\"}}",
                    dev_info.protocol_version,
                    dev_info.firmware_version,
                    dev_info.device_id,
                    dev_info.device_serial_number,
                    dev_info.active_boot_slot,
                    dev_info.reset_reason,
                    dev_info.built_time_utc,
                    dev_info.git_commit,
                )?;

                socket.close();
            }
            Command::ReadMemory => {
                // TODO
            }
            Command::EraseMemory => {
                // TODO
            }
            Command::WriteMemory => {
                // TODO
            }
            Command::CompleteAndReboot => {
                debug!(
                    "UM: scheduling a reobot {} update cycles from now",
                    UPDATE_TICKS_TO_REBOOT
                );
                self.send_status(StatusCode::Success, socket)?;
                self.ticks_until_reboot = Some(UPDATE_TICKS_TO_REBOOT);
            }
            Command::Unknown(_c) => {
                self.send_status(StatusCode::UnknownCommand, socket)?;
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
