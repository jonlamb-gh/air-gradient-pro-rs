use crate::{interruptor::Interruptor, opts::Device};
use anyhow::Result;

mod update;

pub async fn device(cmd: Device, intr: Interruptor) -> Result<()> {
    match cmd {
        Device::Info => todo!(),
        Device::Update(subcmd) => self::update::update(subcmd, intr).await?,
    }
    Ok(())
}
