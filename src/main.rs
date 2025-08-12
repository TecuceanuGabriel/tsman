//! tsman - Main entry point.
//!
//! This module initializes the CLI application, parses command-line arguments,
//! and delegates execution to the appropriate command handler.
mod actions;
mod cli;
mod menu;
mod persistence;
mod tmux;

use anyhow::{Context, Result};
use clap::Parser;

/// Application entry point.
///
/// This function:
/// 1. Parses command-line arguments using [`cli::Args`].
/// 2. Invokes [`actions::handle`] to execute the requested subcommand.
///
/// # Errors
/// Returns an error if:
/// - CLI arguments cannot be parsed (handled internally by `clap`).
/// - The requested action fails (e.g., invalid session name, I/O error).
fn main() -> Result<()> {
    let args = cli::Args::parse();
    actions::handle(args).context("Failed to execute command")?;
    Ok(())
}
