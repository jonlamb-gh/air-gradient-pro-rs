use anyhow::{bail, Result};
use bootloader_support::BootSlot;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use wire_protocols::device::{Command, StatusCode};

#[serde_as]
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeviceInfo {
    pub protocol_version: String,
    pub firmware_version: String,
    pub device_id: u16,
    pub device_serial_number: String,
    #[serde_as(as = "DisplayFromStr")]
    pub active_boot_slot: BootSlot,
    pub reset_reason: String,
    pub built_time_utc: String,
    pub git_commit: String,
}

impl DeviceInfo {
    pub fn from_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
}

pub async fn write_command(cmd: Command, s: &mut TcpStream) -> Result<()> {
    s.write_u32_le(cmd.into()).await?;
    Ok(())
}

pub async fn read_status(s: &mut TcpStream) -> Result<StatusCode> {
    let sc = StatusCode::from(s.read_u32_le().await?);
    if sc.is_success() {
        Ok(sc)
    } else {
        bail!("Err status code = {sc}");
    }
}
