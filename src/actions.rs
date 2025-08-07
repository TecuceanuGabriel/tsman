use std::fs;
use std::process::Command;

use crate::cli::{Args, Commands};
use crate::persistence::*;
use crate::tmux::interface::*;
use crate::tmux::session::Session;
use crate::tui::{self, MenuAction, MenuUi};

use anyhow::{Context, Result};
use regex::Regex;
use shell_escape::escape;

pub fn handle(args: Args) -> Result<()> {
    match args.command {
        Commands::Save { session_name } => save(session_name.as_deref()),
        Commands::Open { session_name } => open(&session_name),
        Commands::Edit { session_name } => edit(session_name.as_deref()),
        Commands::Delete { session_name } => delete(&session_name),
        Commands::Menu { preview } => menu(preview),
    }
}

fn save(session_name: Option<&str>) -> Result<()> {
    let mut current_session =
        get_session().context("Failed to get current session")?;

    if let Some(name) = session_name {
        validate_session_name(name)?;
        current_session.name = name.to_string();
    }

    let yaml = serde_yaml::to_string(&current_session).with_context(|| {
        format!("Failed to serialize session {:#?} to yaml", current_session)
    })?;

    save_session_config(&current_session.name, yaml)
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

fn edit(session_name: Option<&str>) -> Result<()> {
    let path = if let Some(name) = session_name {
        get_config_file_path(name)?
    } else {
        let (name, _) = get_session_info()?;
        get_config_file_path(&name)?
    };

    let path_str = escape(path.as_os_str().to_string_lossy());

    Command::new("sh")
        .arg("-c")
        .arg(format!("$EDITOR {}", path_str))
        .status()?;

    Ok(())
}

fn delete(session_name: &str) -> Result<()> {
    let path = get_config_file_path(&session_name)?;
    fs::remove_file(path)?;
    Ok(())
}

fn menu(show_preview: bool) -> Result<()> {
    let mut terminal = tui::init()?;

    let session_names = list_saved_sessions()?;
    let mut menu_ui = MenuUi::new(session_names, show_preview);
    menu_ui.run(&mut terminal)?;

    tui::restore(terminal)?;

    while let Some(item) = menu_ui.dequeue_action()? {
        match item.action {
            MenuAction::Open => open(&item.selection)?,
            MenuAction::Edit => edit(Some(&item.selection))?,
            MenuAction::Delete => delete(&item.selection)?,
        }
    }

    Ok(())
}
