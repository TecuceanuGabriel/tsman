//! Command dispatcher - routes parsed CLI arguments to the corresponding action.
use std::collections::HashSet;
use std::fs;
use std::process::Command;

use clap::CommandFactory;

use crate::cli::{self, Args, Commands, LayoutCommands};
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

/// Dispatches parsed CLI arguments to the matching subcommand handler.
pub fn handle(args: Args) -> Result<()> {
    match args.command {
        Commands::Save { session_name } => save(session_name.as_deref()),
        Commands::Open { session_name } => open(&session_name),
        Commands::Edit { session_name } => edit(session_name.as_deref()),
        Commands::Reload { session_name } => reload(session_name.as_deref()),
        Commands::Delete { session_name } => delete(&session_name),
        Commands::Menu {
            preview,
            ask_for_confirmation,
        } => menu(preview, ask_for_confirmation),
        Commands::Completions { shell } => {
            completions(shell);
            Ok(())
        }
        Commands::Layout { command } => handle_layout(command),
    }
}

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

/// Saves the tmux session with the given name to disk.
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

/// Restores a saved session, or attaches if it's already active.
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

/// Opens a session's YAML config in `$EDITOR`. Falls back to the current session.
pub fn edit(session_name: Option<&str>) -> Result<()> {
    let path = if let Some(name) = session_name {
        get_config_file_path(StorageKind::Session, name)?
    } else {
        let name = get_session_name()?;
        get_config_file_path(StorageKind::Session, &name)?
    };

    let path_str = escape(path.as_os_str().to_string_lossy());
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} {path_str}"))
        .status()?;

    Ok(())
}

/// Opens a config file (session or layout) in `$EDITOR`.
pub fn edit_config(kind: StorageKind, name: &str) -> Result<()> {
    let path = get_config_file_path(kind, name)?;
    let path_str = escape(path.as_os_str().to_string_lossy());
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} {path_str}"))
        .status()?;

    Ok(())
}

/// Reloads a session from its saved config.
///
/// - If the session is active and we are currently attached to it, uses a
///   temp-session switch to avoid disconnecting the client.
/// - If the session is active but we are not attached, kills and recreates
///   it directly, then attaches.
/// - If the session is not active, opens it fresh (equivalent to `open`).
pub fn reload(session_name: Option<&str>) -> Result<()> {
    let name = match session_name {
        Some(n) => n.to_string(),
        None => {
            anyhow::ensure!(
                std::env::var("TMUX").is_ok(),
                "Reload requires a session name or being inside a tmux \
                 session"
            );
            get_session_name()?
        }
    };

    let yaml = load_config(StorageKind::Session, &name)
        .context("No saved config found for this session")?;

    let session: Session = serde_yaml::from_str(&yaml).with_context(|| {
        format!("Failed to deserialize session from yaml {yaml}")
    })?;

    if is_active_session(&name)? {
        let currently_attached =
            get_session_name().ok().as_deref() == Some(&name);
        reload_session(&session, currently_attached)
            .context("Failed to reload session")?;
    } else {
        restore_session(&session).context("Failed to restore session")?;
    }

    Ok(())
}

/// Deletes a saved session's YAML config from disk.
pub fn delete(session_name: &str) -> Result<()> {
    let path = get_config_file_path(StorageKind::Session, session_name)?;
    fs::remove_file(path)?;
    Ok(())
}

/// Renames a saved config file and updates the name inside the YAML.
pub fn rename(kind: StorageKind, old_name: &str, new_name: &str) -> Result<()> {
    let path = get_config_file_path(kind, old_name)?;
    let mut new_path = path.clone();
    new_path.set_file_name(new_name);
    new_path.set_extension("yaml");
    fs::rename(path, new_path)?;

    let raw_yaml =
        load_config(kind, new_name).context("Failed to read config file")?;
    let mut value: serde_yaml::Value = serde_yaml::from_str(&raw_yaml)
        .with_context(|| format!("Failed to deserialize yaml: {raw_yaml}"))?;
    value["name"] = serde_yaml::Value::String(new_name.to_owned());

    let updated_yaml =
        serde_yaml::to_string(&value).context("Failed to serialize yaml")?;
    save_config(kind, new_name, updated_yaml)
        .context("Failed to save yaml config to disk")?;

    Ok(())
}

fn completions(shell: clap_complete::Shell) {
    clap_complete::generate(
        shell,
        &mut cli::Args::command(),
        "tsman",
        &mut std::io::stdout(),
    );
}

fn menu(show_preview: bool, ask_for_confirmation: bool) -> Result<()> {
    let mut terminal = terminal_utils::init()?;

    let current_session = get_session_name().ok();

    let mut menu = Menu::new(
        get_all_sessions()?,
        show_preview,
        ask_for_confirmation,
        current_session.as_deref(),
        Box::new(DefaultMenuRenderer),
        Box::new(DefaultEventHandler),
        Box::new(DefaultActionDispacher),
    );

    menu.run(&mut terminal)?;

    terminal_utils::restore(terminal)?;

    Ok(())
}

fn get_all_sessions() -> Result<Vec<MenuItem>> {
    let saved_sessions: HashSet<String> =
        list_saved_configs(StorageKind::Session)?
            .into_iter()
            .collect();

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

/// Creates a new tmux session from a saved layout, using `work_dir` for all panes.
pub fn layout_create(
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
                panes: (0..lw.pane_count)
                    .map(|i| Pane {
                        index: i.to_string(),
                        current_command: None,
                        work_dir: work_dir.clone(),
                    })
                    .collect(),
            })
            .collect(),
    };

    restore_session(&session)
        .context("Failed to create session from layout")?;

    Ok(())
}

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

fn layout_delete(layout_name: &str) -> Result<()> {
    let path = get_config_file_path(StorageKind::Layout, layout_name)?;
    fs::remove_file(path)?;
    Ok(())
}

fn layout_edit(layout_name: &str) -> Result<()> {
    let path = get_config_file_path(StorageKind::Layout, layout_name)?;

    let path_str = escape(path.as_os_str().to_string_lossy());
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} {path_str}"))
        .status()?;

    Ok(())
}
