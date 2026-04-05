//! Main entry point - parses CLI arguments and delegates to [`actions::handle`].
mod actions;
mod cli;
mod config;
mod menu;
mod persistence;
mod terminal_utils;
mod tmux;
mod util;

use anyhow::{Context, Result};
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    actions::handle(args).context("Failed to execute command")?;
    Ok(())
}
