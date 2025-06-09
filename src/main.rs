mod actions;
mod cli;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    actions::handle(args);
}
