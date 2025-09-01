//! Command dispatcher for `tsman` CLI.
//!
//! This module takes parsed CLI arguments and executes the corresponding
//! tmux session management action.
use std::collections::HashSet;
use std::fs;
use std::process::Command;

use crate::cli::{Args, Commands};
use crate::menu::{MenuItem, MenuUi};
use crate::persistence::*;
use crate::terminal_utils;
use crate::tmux::interface::*;
use crate::tmux::session::Session;

use anyhow::{Context, Result};
use shell_escape::escape;

/// Handles CLI arguments and dispatches to the appropriate subcommand handler.
///
/// This is the main entry point for executing commands like:
/// - `save`
/// - `open`
/// - `edit`
/// - `delete`
/// - `menu`
///
/// # Arguments
/// * `args` – Parsed CLI arguments from [`crate::cli`].
///
/// # Errors
/// Returns an error if the underlying command fails.
pub fn handle(args: Args) -> Result<()> {
    match args.command {
        Commands::Save { session_name } => save(session_name.as_deref()),
        Commands::Open { session_name } => open(&session_name),
        Commands::Edit { session_name } => edit(session_name.as_deref()),
        Commands::Delete { session_name } => delete(&session_name),
        Commands::Menu {
            preview,
            ask_for_confirmation,
        } => menu(preview, ask_for_confirmation),
    }
}

/// Saves the current tmux session configuration.
///
/// If `session_name` is provided, renames the saved session to that name.
///
/// # Arguments
/// * `session_name` – Optional override for the current session name.
///
/// # Errors
/// Returns an error if:
/// - The current tmux session cannot be retrieved.
/// - YAML serialization fails.
/// - The configuration cannot be saved.
fn save(session_name: Option<&str>) -> Result<()> {
    let mut current_session =
        get_session(None).context("Failed to get current session")?;

    if let Some(name) = session_name {
        current_session.name = name.to_string();
    }

    let yaml = serde_yaml::to_string(&current_session).with_context(|| {
        format!("Failed to serialize session {current_session:#?} to yaml")
    })?;

    save_session_config(&current_session.name, yaml)
        .context("Failed to save yaml config to disk")?;

    Ok(())
}

/// Saves a specific tmux session configuration.
///
/// Similar to [`save`] but explicitly targets a given session by name.
///
/// # Arguments
/// * `session_name` – Name of the session to save.
///
/// # Errors
/// Same as [`save`].
pub fn save_target(session_name: &str) -> Result<()> {
    let current_session = get_session(Some(session_name))
        .context("Failed to get current session")?;

    let yaml = serde_yaml::to_string(&current_session).with_context(|| {
        format!("Failed to serialize session {current_session:#?} to yaml")
    })?;

    save_session_config(&current_session.name, yaml)
        .context("Failed to save yaml config to disk")?;

    Ok(())
}

/// Opens (restores) a tmux session.
///
/// If the session is already active, attaches to it. Otherwise, loads it from
/// the saved YAML config and restores it.
///
/// # Arguments
/// * `session_name` – Name of the session to open.
///
/// # Errors
/// Returns an error if:
/// - The session cannot be found.
/// - YAML deserialization fails.
/// - tmux restoration commands fail.
pub fn open(session_name: &str) -> Result<()> {
    if is_active_session(session_name)? {
        attach_to_session(session_name)?;
        return Ok(());
    }

    let yaml = load_session_from_config(session_name)
        .context("Failed to read session from config file")?;

    let session: Session = serde_yaml::from_str(&yaml).with_context(|| {
        format!("Failed to deserialize session from yaml {yaml}")
    })?;

    restore_session(&session).context("Failed to restore session")?;

    Ok(())
}

/// Opens the session configuration file in `$EDITOR`.
///
/// # Arguments
///
/// * `session_name` – Optional name of the session to edit. If omitted, edits
///   the current active session.
///
/// # Errors
/// Returns an error if:
/// - The session name cannot be determined.
/// - The editor command fails.
pub fn edit(session_name: Option<&str>) -> Result<()> {
    let path = if let Some(name) = session_name {
        get_config_file_path(name)?
    } else {
        let name = get_session_name()?;
        get_config_file_path(&name)?
    };

    let path_str = escape(path.as_os_str().to_string_lossy());

    Command::new("sh")
        .arg("-c")
        .arg(format!("$EDITOR {path_str}"))
        .status()?;

    Ok(())
}

/// Deletes a saved session configuration file.
///
/// # Arguments
/// * `session_name` – Name of the session to delete.
///
/// # Errors
/// Returns an error if the file cannot be removed.
pub fn delete(session_name: &str) -> Result<()> {
    let path = get_config_file_path(session_name)?;
    fs::remove_file(path)?;
    Ok(())
}

/// Launches an interactive menu for managing tmux sessions.
///
/// The menu displays all saved and active sessions and allows the user to:
/// - Save
/// - Open
/// - Edit
/// - Delete
/// - Close
///
/// # Arguments
/// * `show_preview` – Whether to show session previews.
/// * `ask_for_confirmation` – Whether to prompt before destructive actions.
///
/// # Errors
/// Returns an error if the menu fails to initialize, display, or perform
/// any action.
fn menu(show_preview: bool, ask_for_confirmation: bool) -> Result<()> {
    let mut terminal = terminal_utils::init()?;

    let mut menu_ui =
        MenuUi::new(get_all_sessions()?, show_preview, ask_for_confirmation);
    menu_ui.run(&mut terminal)?;

    terminal_utils::restore(terminal)?;

    Ok(())
}

/// Retrieves all sessions (saved and/or active) as menu items.
///
/// Performs a union of:
/// - Saved sessions from [`list_saved_sessions`]
/// - Active sessions from [`list_active_sessions`]
///
/// # Returns
/// A vector of [`MenuItem`] with metadata indicating saved/active status.
///
/// # Errors
/// Returns an error if listing sessions fails.
fn get_all_sessions() -> Result<Vec<MenuItem>> {
    let saved_sessions: HashSet<String> =
        list_saved_sessions()?.into_iter().collect();

    let active_sessions: HashSet<String> =
        list_active_sessions()?.into_iter().collect();

    let union: HashSet<_> =
        saved_sessions.union(&active_sessions).cloned().collect();

    let all_sessions: Vec<MenuItem> = union
        .into_iter()
        .map(|name| {
            MenuItem::new(
                name.clone(),
                saved_sessions.contains(&name),
                active_sessions.contains(&name),
            )
        })
        .collect();

    Ok(all_sessions)
}
