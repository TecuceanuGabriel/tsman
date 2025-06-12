use std::{fs, path};

use serde::Serialize;

use crate::cli::{Args, Commands};
use crate::persistence::*;
use crate::tmux_interface::*;

const BASE_PATH: &str = "";

fn new(_session_name: &str) {
    todo!();
}

fn save(session_name: &str) -> Result<(), TmuxError> {
    let mut current_session = get_session()?;
    current_session.name = session_name.to_string();

    let yaml = serde_yaml::to_string(&current_session).unwrap();

    save_session_config(session_name, yaml);

    Ok(())
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

pub fn handle(args: Args) -> Result<(), TmuxError> {
    match args.command {
        Commands::New { session_name } => Ok(new(&session_name)),
        Commands::Save { session_name } => save(&session_name),
        Commands::Open { session_name } => Ok(open(&session_name)),
        Commands::Edit { session_name } => Ok(edit(&session_name)),
        Commands::Menu => Ok(menu()),
    }
}
