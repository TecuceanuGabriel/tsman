//! Command dispatcher for `tsman` CLI.
//!
//! This module takes parsed CLI arguments and executes the corresponding
//! tmux session management action.
use std::collections::HashSet;
use std::fs;
use std::process::Command;

use crate::cli::{Args, Commands, LayoutCommands};
use crate::menu::Menu;
use crate::menu::action_dispatcher::DefaultActionDispacher;
use crate::menu::event_handler::DefaultEventHandler;
use crate::menu::item::MenuItem;
use crate::menu::renderer::DefaultMenuRenderer;
use crate::persistence::*;
use crate::terminal_utils;
use crate::tmux::interface::*;
use crate::tmux::layout::Layout;
use crate::tmux::session::{Pane, Session, Window};

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
        Commands::Layout { command } => handle_layout(command),
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

    save_config(StorageKind::Session, &current_session.name, yaml)
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

    save_config(StorageKind::Session, &current_session.name, yaml)
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

    let yaml = load_config(StorageKind::Session, session_name)
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
        get_config_file_path(StorageKind::Session, name)?
    } else {
        let name = get_session_name()?;
        get_config_file_path(StorageKind::Session, &name)?
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
    let path = get_config_file_path(StorageKind::Session, session_name)?;
    fs::remove_file(path)?;
    Ok(())
}

pub fn rename(session_name: &str, new_name: &str) -> Result<()> {
    let path = get_config_file_path(StorageKind::Session, session_name)?;
    let mut new_path = path.clone();
    new_path.set_file_name(new_name);
    new_path.set_extension("yaml");
    fs::rename(path, new_path)?;

    let old_yaml = load_config(StorageKind::Session, new_name)
        .context("Failed to read session from config file")?;
    let mut session: Session =
        serde_yaml::from_str(&old_yaml).with_context(|| {
            format!("Failed to deserialize session from yaml {old_yaml}")
        })?;
    session.name = new_name.to_owned();

    let updated_yaml = serde_yaml::to_string(&session).with_context(|| {
        format!("Failed to serialize session {session:#?} to yaml")
    })?;
    save_config(StorageKind::Session, &session.name, updated_yaml)
        .context("Failed to save yaml config to disk")?;

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

    let mut menu = Menu::new(
        get_all_sessions()?,
        show_preview,
        ask_for_confirmation,
        Box::new(DefaultMenuRenderer),
        Box::new(DefaultEventHandler),
        Box::new(DefaultActionDispacher),
    );

    menu.run(&mut terminal)?;

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
        list_saved_configs(StorageKind::Session)?.into_iter().collect();

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

fn handle_layout(command: LayoutCommands) -> Result<()> {
    match command {
        LayoutCommands::Save { layout_name } => {
            layout_save(layout_name.as_deref())
        }
        LayoutCommands::Create {
            layout_name,
            work_dir,
            session_name,
        } => layout_create(&layout_name, &work_dir, session_name.as_deref()),
        LayoutCommands::List => layout_list(),
        LayoutCommands::Delete { layout_name } => layout_delete(&layout_name),
        LayoutCommands::Edit { layout_name } => layout_edit(&layout_name),
    }
}

/// Saves the current tmux session as a layout template.
///
/// Captures the window/pane structure without working directories.
fn layout_save(layout_name: Option<&str>) -> Result<()> {
    let current_session =
        get_session(None).context("Failed to get current session")?;

    let mut layout = Layout::from(&current_session);

    if let Some(name) = layout_name {
        layout.name = name.to_string();
    }

    let yaml = serde_yaml::to_string(&layout).with_context(|| {
        format!("Failed to serialize layout {layout:#?} to yaml")
    })?;

    save_config(StorageKind::Layout, &layout.name, yaml)
        .context("Failed to save layout config to disk")?;

    Ok(())
}

/// Creates a new tmux session from a saved layout template.
///
/// All panes start in the specified working directory.
fn layout_create(
    layout_name: &str,
    work_dir: &str,
    session_name: Option<&str>,
) -> Result<()> {
    let work_dir = std::fs::canonicalize(work_dir)
        .with_context(|| format!("Invalid working directory: {work_dir}"))?
        .to_string_lossy()
        .to_string();

    let yaml = load_config(StorageKind::Layout, layout_name)
        .context("Failed to read layout from config file")?;

    let layout: Layout = serde_yaml::from_str(&yaml).with_context(|| {
        format!("Failed to deserialize layout from yaml {yaml}")
    })?;

    let name = session_name.unwrap_or(layout_name).to_string();

    if is_active_session(&name)? {
        anyhow::bail!("Session '{name}' already exists");
    }

    let session = Session {
        name,
        work_dir: work_dir.clone(),
        windows: layout
            .windows
            .iter()
            .map(|lw| Window {
                index: lw.index.clone(),
                name: lw.name.clone(),
                layout: lw.layout.clone(),
                panes: lw
                    .panes
                    .iter()
                    .map(|lp| Pane {
                        index: lp.index.clone(),
                        current_command: lp.current_command.clone(),
                        work_dir: work_dir.clone(),
                    })
                    .collect(),
            })
            .collect(),
    };

    restore_session(&session).context("Failed to create session from layout")?;

    Ok(())
}

/// Lists all saved layout templates.
fn layout_list() -> Result<()> {
    let layouts = list_saved_configs(StorageKind::Layout)?;
    if layouts.is_empty() {
        println!("No saved layouts.");
    } else {
        for name in layouts {
            println!("{name}");
        }
    }
    Ok(())
}

/// Deletes a saved layout configuration file.
fn layout_delete(layout_name: &str) -> Result<()> {
    let path = get_config_file_path(StorageKind::Layout, layout_name)?;
    fs::remove_file(path)?;
    Ok(())
}

/// Opens a layout configuration file in `$EDITOR`.
fn layout_edit(layout_name: &str) -> Result<()> {
    let path = get_config_file_path(StorageKind::Layout, layout_name)?;

    let path_str = escape(path.as_os_str().to_string_lossy());

    Command::new("sh")
        .arg("-c")
        .arg(format!("$EDITOR {path_str}"))
        .status()?;

    Ok(())
}
