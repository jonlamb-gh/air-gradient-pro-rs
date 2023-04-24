use crate::{interruptor::Interruptor, opts::Device};
use anyhow::Result;

mod info;
mod update;

pub async fn device(cmd: Device, intr: Interruptor) -> Result<()> {
    match cmd {
        Device::Info(subcmd) => self::info::info(subcmd, intr).await?,
        Device::Update(subcmd) => self::update::update(subcmd, intr).await?,
    }
    Ok(())
}
