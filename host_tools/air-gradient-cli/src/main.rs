//#![deny(warnings, clippy::all)]

use crate::{interruptor::Interruptor, opts::Command};
use anyhow::Result;
use clap::Parser;

mod command;
mod interruptor;
mod measurement;
mod opts;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = opts::Opts::parse();

    try_init_tracing_subscriber()?;

    let intr = Interruptor::new();
    let interruptor = intr.clone();
    ctrlc::set_handler(move || {
        if intr.is_set() {
            let exit_code = if cfg!(target_family = "unix") {
                // 128 (fatal error signal "n") + 2 (control-c is fatal error signal 2)
                130
            } else {
                // Windows code 3221225786
                // -1073741510 == C000013A
                -1073741510
            };
            std::process::exit(exit_code);
        } else {
            intr.set();
        }
    })?;

    let mut join_handle = tokio::spawn(async move {
        match opts.command {
            Command::Listen(c) => command::listen(c, interruptor).await,
            Command::InfluxRelay(c) => command::influx_relay(c, interruptor).await,
        }
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::debug!("User signaled shutdown");
        }
        res = &mut join_handle => {
            let _res = res?;
        }
    };

    join_handle.await??;

    Ok(())
}

fn try_init_tracing_subscriber() -> Result<()> {
    use tracing_subscriber::util::SubscriberInitExt;
    let builder = tracing_subscriber::fmt::Subscriber::builder();
    let env_filter = std::env::var(tracing_subscriber::EnvFilter::DEFAULT_ENV)
        .map(tracing_subscriber::EnvFilter::new)
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(format!(
                "{}={}",
                env!("CARGO_PKG_NAME").replace('-', "_"),
                tracing::Level::WARN
            ))
        });
    let builder = builder.with_env_filter(env_filter);
    let subscriber = builder.finish();
    subscriber.try_init()?;
    Ok(())
}
