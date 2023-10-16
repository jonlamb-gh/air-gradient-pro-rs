use crate::{
    device_util::{self, DeviceInfo},
    interruptor::Interruptor,
    opts::{DeviceInfo as DeviceInfoOps, Format},
};
use anyhow::Result;
use std::net;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpStream,
};
use tracing::debug;
use wire_protocols::device::Command;

pub async fn info(cmd: DeviceInfoOps, _intr: Interruptor) -> Result<()> {
    if cmd.common.format.is_text() && cmd.common.verbose {
        println!(
            "Requesting device info from {}:{}",
            cmd.common.address, cmd.common.port
        );
    }

    let s = net::TcpStream::connect((cmd.common.address.as_str(), cmd.common.port))?;
    s.set_nonblocking(true)?;
    let mut stream = TcpStream::from_std(s)?;

    debug!("Requesting device info");
    device_util::write_command(Command::Info, &mut stream).await?;
    let status = device_util::read_status(&mut stream).await?;

    if cmd.common.format.is_text() && cmd.common.verbose {
        println!("Status: {status}");
    }

    let mut buf_stream = BufReader::new(stream);

    let mut info_str = String::new();
    let _info_len = buf_stream.read_line(&mut info_str).await?;

    let info = DeviceInfo::from_json(&info_str)?;
    match cmd.common.format {
        Format::Text => {
            if cmd.field_names.is_empty() {
                println!("{info:#?}")
            } else {
                let map = info.into_field_names_and_values();
                for field in cmd.field_names.into_iter() {
                    if let Some(v) = map.get(&field) {
                        println!("{v}");
                    }
                }
            }
        }
        // TODO - handle field_names opts
        Format::Json => println!("{}", serde_json::to_string_pretty(&info)?),
    }

    Ok(())
}
