use crate::{
    device_util::{self, DeviceInfo},
    interruptor::Interruptor,
    opts::{CommonDeviceOpts, Format},
};
use anyhow::Result;
use std::net;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpStream,
};
use tracing::debug;
use wire_protocols::device::Command;

pub async fn info(cmd: CommonDeviceOpts, _intr: Interruptor) -> Result<()> {
    if cmd.format.is_text() {
        println!("Requesting device info from {}:{}", cmd.address, cmd.port);
    }

    let s = net::TcpStream::connect((cmd.address.as_str(), cmd.port))?;
    s.set_nonblocking(true)?;
    let mut stream = TcpStream::from_std(s)?;

    debug!("Requesting device info");
    device_util::write_command(Command::Info, &mut stream).await?;
    let status = device_util::read_status(&mut stream).await?;

    if cmd.format.is_text() {
        println!("Status: {status}");
    }

    let mut buf_stream = BufReader::new(stream);

    let mut info_str = String::new();
    let _info_len = buf_stream.read_line(&mut info_str).await?;

    let info = DeviceInfo::from_json(&info_str)?;
    match cmd.format {
        Format::Text => println!("{info:#?}"),
        Format::Json => println!("{}", serde_json::to_string_pretty(&info)?),
    }

    Ok(())
}
