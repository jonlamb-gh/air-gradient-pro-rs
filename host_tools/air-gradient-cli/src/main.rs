use crate::{interruptor::Interruptor, opts::Command};
use anyhow::Result;
use clap::Parser;

mod command;
mod interruptor;
mod opts;

fn main() -> Result<()> {
    let opts = opts::Opts::parse();

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

    match opts.command {
        Command::Listen(c) => command::listen(c, interruptor)?,
    }

    Ok(())
}
