mod actions;
mod cli;
mod persistence;
mod tmux_interface;

use anyhow::{Context, Result};
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    actions::handle(args).context("Failed to execute command")?;
    Ok(())
}
