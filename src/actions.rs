use std::process::Command;

use crate::cli::{Args, Commands};
use crate::persistence::*;
use crate::tmux_interface::*;
use crate::tui::{self, MenuUi};

use anyhow::{Context, Result};
use regex::Regex;
use shell_escape::escape;

pub fn handle(args: Args) -> Result<()> {
    match args.command {
        Commands::Save { session_name } => save(&session_name),
        Commands::Open { session_name } => open(&session_name),
        Commands::Edit => edit(),
        Commands::Menu => menu(),
    }
}

fn save(session_name: &str) -> Result<()> {
    validate_session_name(session_name)?;

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

fn validate_session_name(name: &str) -> Result<()> {
    let re: Regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    if !re.is_match(name) {
        anyhow::bail!("Invalid session name. Only [a-zA-Z0-9_-] allowed");
    }
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

fn edit() -> Result<()> {
    if let Some(session_name) = querry_sessions()? {
        let path = get_config_file_path(&session_name)?;
        let path_str = escape(path.as_os_str().to_string_lossy());

        Command::new("sh")
            .arg("-c")
            .arg(format!("$EDITOR {}", path_str))
            .status()?;
    }

    Ok(())
}

fn menu() -> Result<()> {
    if let Some(session_name) = querry_sessions()? {
        open(&session_name)?;
    }

    Ok(())
}

fn querry_sessions() -> Result<Option<String>> {
    let mut terminal = tui::init()?;

    let session_names = list_saved_sessions()?;
    let mut menu_ui = MenuUi::new(session_names);
    menu_ui.run(&mut terminal)?;

    tui::restore(terminal)?;

    Ok(menu_ui.get_selected())
}
