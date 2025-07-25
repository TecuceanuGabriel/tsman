use crate::cli::{Args, Commands};
use crate::persistence::*;
use crate::tmux_interface::*;

use anyhow::{Context, Result};

pub fn handle(args: Args) -> Result<()> {
    match args.command {
        Commands::New { session_name } => new(&session_name),
        Commands::Save { session_name } => save(&session_name),
        Commands::Open { session_name } => open(&session_name),
        Commands::Edit { session_name } => edit(&session_name),
        Commands::Menu => menu(),
    }
}

fn new(_session_name: &str) -> Result<()> {
    todo!();
}

fn save(session_name: &str) -> Result<()> {
    let mut current_session =
        get_session().context("Failed to get current session")?;

    current_session.name = session_name.to_string();

    let yaml = serde_yaml::to_string(&current_session).with_context(|| {
        format!("Failed to serialize session {:#?} to yaml", current_session)
    })?;

    save_session_config(session_name, yaml)
        .context("Failed to save yaml config to disk")?;

    Ok(())
}

fn open(session_name: &str) -> Result<()> {
    let yaml = load_session_from_config(session_name)
        .context("Failed to read session from config file")?;

    let session: Session = serde_yaml::from_str(&yaml).with_context(|| {
        format!("Failed to deserialize session from yaml {}", yaml)
    })?;

    restore_session(&session).context("Failed to restore session")?;

    Ok(())
}

fn edit(_session_name: &str) -> Result<()> {
    todo!();
}

fn menu() -> Result<()> {
    todo!();
}
