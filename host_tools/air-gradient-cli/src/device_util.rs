use anyhow::{bail, Result};
use bootloader_support::BootSlot;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::debug;
use wire_protocols::device::{Command, StatusCode};

#[serde_as]
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct DeviceInfo {
    pub protocol_version: String,
    pub firmware_version: String,
    pub device_id: u16,
    pub device_serial_number: String,
    pub mac_address: [u8; 6],
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

    // TODO - use serde_transcode to do this instead of manually
    pub fn into_field_names_and_values(self) -> serde_json::Map<String, serde_json::Value> {
        vec![
            ("protocol_version".to_owned(), self.protocol_version.into()),
            ("firmware_version".to_owned(), self.firmware_version.into()),
            ("device_id".to_owned(), self.device_id.into()),
            (
                "device_serial_number".to_owned(),
                self.device_serial_number.into(),
            ),
            (
                "mac_address".to_owned(),
                fmt_mac_addr(&self.mac_address).into(),
            ),
            (
                "active_boot_slot".to_owned(),
                self.active_boot_slot.to_string().into(),
            ),
            ("reset_reason".to_owned(), self.reset_reason.into()),
            ("built_time_utc".to_owned(), self.built_time_utc.into()),
            ("git_commit".to_owned(), self.git_commit.into()),
        ]
        .into_iter()
        .collect()
    }
}

pub async fn write_command(cmd: Command, s: &mut TcpStream) -> Result<()> {
    s.write_u32_le(cmd.into()).await?;
    Ok(())
}

pub async fn read_status(s: &mut TcpStream) -> Result<StatusCode> {
    let sc = StatusCode::from(s.read_u32_le().await?);
    debug!("Read status {sc}");
    if sc.is_success() {
        Ok(sc)
    } else {
        bail!("Err status code = {sc}");
    }
}

fn fmt_mac_addr(bytes: &[u8; 6]) -> String {
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]
    )
}
