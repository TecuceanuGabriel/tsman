mod actions;
mod cli;
mod tmux_interface;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    actions::handle(args);
}
