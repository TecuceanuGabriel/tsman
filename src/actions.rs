use crate::cli::{Args, Commands};

use crate::tmux_interface::*;

fn new(_session_name: &str) {
    todo!();
}

fn save(_session_name: &str) {
    match get_session() {
        Ok(current_session) => {
            println!("Current session: {:#?}", current_session);
        }
        Err(err) => {
            eprintln!("Failed to get session: {:#?}", err);
        }
    }
}

fn open(_session_name: &str) {
    todo!();
}

fn edit(_session_name: &str) {
    todo!();
}

fn menu() {
    todo!();
}

pub fn handle(args: Args) {
    match args.command {
        Commands::New { session_name } => new(&session_name),
        Commands::Save { session_name } => save(&session_name),
        Commands::Open { session_name } => open(&session_name),
        Commands::Edit { session_name } => edit(&session_name),
        Commands::Menu => menu(),
    }
}
