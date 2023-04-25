use crate::{device_util, interruptor::Interruptor, opts::CommonDeviceOpts};
use anyhow::Result;
use std::net;
use tokio::net::TcpStream;
use tracing::debug;
use wire_protocols::device::Command;

pub async fn reboot(cmd: CommonDeviceOpts, _intr: Interruptor) -> Result<()> {
    if cmd.format.is_text() {
        println!("Rebooting device {}:{}", cmd.address, cmd.port);
    }

    let s = net::TcpStream::connect((cmd.address.as_str(), cmd.port))?;
    s.set_nonblocking(true)?;
    let mut stream = TcpStream::from_std(s)?;

    debug!("Requesting device reboot");
    device_util::write_command(Command::CompleteAndReboot, &mut stream).await?;
    let status = device_util::read_status(&mut stream).await?;

    if cmd.format.is_text() {
        println!("Status: {status}");
    }

    Ok(())
}
