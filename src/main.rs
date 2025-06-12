mod actions;
mod cli;
mod persistence;
mod tmux_interface;

use clap::Parser;
use tmux_interface::TmuxError;

fn main() -> Result<(), TmuxError> {
    let args = cli::Args::parse();
    actions::handle(args)?;
    Ok(())
}
