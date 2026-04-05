//! Command dispatcher - routes parsed CLI arguments to the corresponding action.
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

use clap::CommandFactory;

use crate::cli::{self, Args, Commands, LayoutCommands};
use crate::config::Config;
use crate::menu::Menu;
use crate::menu::action_dispatcher::DefaultActionDispacher;
use crate::menu::event_handler::DefaultEventHandler;
use crate::menu::item::MenuItem;
use crate::menu::renderer::DefaultMenuRenderer;
use crate::menu::ui_flags::UiFlags;
use crate::persistence::{Persistence, StorageKind};
use crate::terminal_utils;
use crate::tmux::interface::*;
use crate::tmux::layout::Layout;
use crate::tmux::session::{Pane, Session, Window};
use dirs::home_dir;

use anyhow::{Context, Result};
use shell_escape::escape;

/// Dispatches parsed CLI arguments to the matching subcommand handler.
pub fn handle(args: Args) -> Result<()> {
    let config = Config::load()?;
    let persistence = Persistence::new(&config.storage)?;

    match args.command {
        Commands::Save { session_name } => {
            save(session_name.as_deref(), &persistence)
        }
        Commands::Open { session_name } => open(&session_name, &persistence),
        Commands::Edit { session_name } => {
            edit(session_name.as_deref(), &persistence)
        }
        Commands::Reload { session_name } => {
            reload(session_name.as_deref(), &persistence)
        }
        Commands::Delete { session_name } => {
            delete(&session_name, &persistence)
        }
        Commands::Menu {
            preview,
            ask_for_confirmation,
        } => {
            let show_preview = preview || config.menu.preview;
            let confirm =
                ask_for_confirmation || config.menu.ask_for_confirmation;
            menu(
                show_preview,
                confirm,
                config.menu.show_key_presses,
                persistence,
            )
        }
        Commands::Completions { shell } => {
            completions(shell);
            Ok(())
        }
        Commands::Init => init(),
        Commands::Layout { command } => handle_layout(command, &persistence),
    }
}

fn save(session_name: Option<&str>, persistence: &Persistence) -> Result<()> {
    let mut current_session =
        get_session(None).context("Failed to get current session")?;

    if let Some(name) = session_name {
        current_session.name = name.to_string();
    }

    let yaml = serde_yaml::to_string(&current_session).with_context(|| {
        format!("Failed to serialize session {current_session:#?} to yaml")
    })?;

    persistence
        .save_config(StorageKind::Session, &current_session.name, yaml)
        .context("Failed to save yaml config to disk")?;

    Ok(())
}

/// Saves the tmux session with the given name to disk.
pub fn save_target(
    session_name: &str,
    persistence: &Persistence,
) -> Result<()> {
    let current_session = get_session(Some(session_name))
        .context("Failed to get current session")?;

    let yaml = serde_yaml::to_string(&current_session).with_context(|| {
        format!("Failed to serialize session {current_session:#?} to yaml")
    })?;

    persistence
        .save_config(StorageKind::Session, &current_session.name, yaml)
        .context("Failed to save yaml config to disk")?;

    Ok(())
}

/// Restores a saved session, or attaches if it's already active.
pub fn open(session_name: &str, persistence: &Persistence) -> Result<()> {
    if is_active_session(session_name)? {
        attach_to_session(session_name)?;
        return Ok(());
    }

    let yaml = persistence
        .load_config(StorageKind::Session, session_name)
        .context("Failed to read session from config file")?;

    let session: Session = serde_yaml::from_str(&yaml).with_context(|| {
        format!("Failed to deserialize session from yaml {yaml}")
    })?;

    restore_session(&session).context("Failed to restore session")?;

    Ok(())
}

/// Opens a session's YAML config in `$EDITOR`. Falls back to the current session.
pub fn edit(
    session_name: Option<&str>,
    persistence: &Persistence,
) -> Result<()> {
    let path = if let Some(name) = session_name {
        persistence.get_config_file_path(StorageKind::Session, name)?
    } else {
        let name = get_session_name()?;
        persistence.get_config_file_path(StorageKind::Session, &name)?
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
pub fn edit_config(
    persistence: &Persistence,
    kind: StorageKind,
    name: &str,
) -> Result<()> {
    let path = persistence.get_config_file_path(kind, name)?;
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
pub fn reload(
    session_name: Option<&str>,
    persistence: &Persistence,
) -> Result<()> {
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

    let yaml = persistence
        .load_config(StorageKind::Session, &name)
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
pub fn delete(session_name: &str, persistence: &Persistence) -> Result<()> {
    let path =
        persistence.get_config_file_path(StorageKind::Session, session_name)?;
    fs::remove_file(path)?;
    Ok(())
}

/// Renames a saved config file and updates the name inside the YAML.
pub fn rename(
    persistence: &Persistence,
    kind: StorageKind,
    old_name: &str,
    new_name: &str,
) -> Result<()> {
    let path = persistence.get_config_file_path(kind, old_name)?;
    let mut new_path = path.clone();
    new_path.set_file_name(new_name);
    new_path.set_extension("yaml");
    fs::rename(path, new_path)?;

    let raw_yaml = persistence
        .load_config(kind, new_name)
        .context("Failed to read config file")?;
    let mut value: serde_yaml::Value = serde_yaml::from_str(&raw_yaml)
        .with_context(|| format!("Failed to deserialize yaml: {raw_yaml}"))?;
    value["name"] = serde_yaml::Value::String(new_name.to_owned());

    let updated_yaml =
        serde_yaml::to_string(&value).context("Failed to serialize yaml")?;
    persistence
        .save_config(kind, new_name, updated_yaml)
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

fn menu(
    show_preview: bool,
    ask_for_confirmation: bool,
    show_key_presses: bool,
    persistence: Persistence,
) -> Result<()> {
    let mut terminal = terminal_utils::init()?;

    let current_session = get_session_name().ok();

    let mut menu = Menu::new(
        get_all_sessions(&persistence)?,
        UiFlags::new(ask_for_confirmation, show_preview, show_key_presses),
        current_session.as_deref(),
        persistence,
        Box::new(DefaultMenuRenderer),
        Box::new(DefaultEventHandler),
        Box::new(DefaultActionDispacher),
    );

    menu.run(&mut terminal)?;

    terminal_utils::restore(terminal)?;

    Ok(())
}

fn get_all_sessions(persistence: &Persistence) -> Result<Vec<MenuItem>> {
    let saved_sessions: HashSet<String> = persistence
        .list_saved_configs(StorageKind::Session)?
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

fn handle_layout(
    command: LayoutCommands,
    persistence: &Persistence,
) -> Result<()> {
    match command {
        LayoutCommands::Save { layout_name } => {
            layout_save(layout_name.as_deref(), persistence)
        }
        LayoutCommands::Create {
            layout_name,
            work_dir,
            session_name,
        } => layout_create(
            &layout_name,
            &work_dir,
            session_name.as_deref(),
            persistence,
        ),
        LayoutCommands::List => layout_list(persistence),
        LayoutCommands::Delete { layout_name } => {
            layout_delete(&layout_name, persistence)
        }
        LayoutCommands::Edit { layout_name } => {
            layout_edit(&layout_name, persistence)
        }
    }
}

fn layout_save(
    layout_name: Option<&str>,
    persistence: &Persistence,
) -> Result<()> {
    let current_session =
        get_session(None).context("Failed to get current session")?;

    let mut layout = Layout::from(&current_session);

    if let Some(name) = layout_name {
        layout.name = name.to_string();
    }

    let yaml = serde_yaml::to_string(&layout).with_context(|| {
        format!("Failed to serialize layout {layout:#?} to yaml")
    })?;

    persistence
        .save_config(StorageKind::Layout, &layout.name, yaml)
        .context("Failed to save layout config to disk")?;

    Ok(())
}

/// Creates a new tmux session from a saved layout, using `work_dir` for all panes.
pub fn layout_create(
    layout_name: &str,
    work_dir: &str,
    session_name: Option<&str>,
    persistence: &Persistence,
) -> Result<()> {
    let work_dir = std::fs::canonicalize(work_dir)
        .with_context(|| format!("Invalid working directory: {work_dir}"))?
        .to_string_lossy()
        .to_string();

    let yaml = persistence
        .load_config(StorageKind::Layout, layout_name)
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

fn layout_list(persistence: &Persistence) -> Result<()> {
    let layouts = persistence.list_saved_configs(StorageKind::Layout)?;
    if layouts.is_empty() {
        println!("No saved layouts.");
    } else {
        for name in layouts {
            println!("{name}");
        }
    }
    Ok(())
}

fn layout_delete(layout_name: &str, persistence: &Persistence) -> Result<()> {
    let path =
        persistence.get_config_file_path(StorageKind::Layout, layout_name)?;
    fs::remove_file(path)?;
    Ok(())
}

fn layout_edit(layout_name: &str, persistence: &Persistence) -> Result<()> {
    let path =
        persistence.get_config_file_path(StorageKind::Layout, layout_name)?;

    let path_str = escape(path.as_os_str().to_string_lossy());
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} {path_str}"))
        .status()?;

    Ok(())
}

fn init() -> Result<()> {
    let home = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine HOME directory"))?;

    let default_sessions = home.join(".config").join(".tsessions");
    let default_layouts = home.join(".config").join(".tlayouts");

    println!("Initializing tsman — press Enter to accept defaults.\n");

    let sessions_dir = prompt_path(
        &format!("Sessions directory [{}]: ", default_sessions.display()),
        &default_sessions,
    )?;
    let layouts_dir = prompt_path(
        &format!("Layouts directory [{}]: ", default_layouts.display()),
        &default_layouts,
    )?;

    println!();
    let preview = prompt_bool("Enable preview pane by default? [Y/n]: ")?;
    let ask_for_confirmation =
        prompt_bool("Prompt for confirmation before deleting? [Y/n]: ")?;
    let show_key_presses =
        prompt_bool("Show key press hints in menu? [Y/n]: ")?;

    // Check for existing config before writing anything.
    let config_dir = home.join(".config").join("tsman");
    let config_path = config_dir.join("config.toml");
    if config_path.exists() {
        let overwrite = prompt_bool(&format!(
            "\nConfig already exists at {}. Overwrite? [y/N]: ",
            config_path.display()
        ))?;
        if !overwrite {
            println!("Aborted.");
            return Ok(());
        }
    }

    fs::create_dir_all(&sessions_dir).with_context(|| {
        format!("Failed to create {}", sessions_dir.display())
    })?;
    fs::create_dir_all(&layouts_dir).with_context(|| {
        format!("Failed to create {}", layouts_dir.display())
    })?;
    fs::create_dir_all(&config_dir).with_context(|| {
        format!("Failed to create {}", config_dir.display())
    })?;

    let sessions_str = sessions_dir.to_string_lossy();
    let layouts_str = layouts_dir.to_string_lossy();
    let toml = format!(
        "[menu]\n\
         preview = {preview}\n\
         ask_for_confirmation = {ask_for_confirmation}\n\
         show_key_presses = {show_key_presses}\n\
         \n\
         [storage]\n\
         sessions_dir = \"{sessions_str}\"\n\
         layouts_dir = \"{layouts_str}\"\n"
    );

    fs::write(&config_path, toml)?;
    println!("\nDone! Config written to {}", config_path.display());

    Ok(())
}

fn prompt_path(prompt: &str, default: &std::path::Path) -> Result<PathBuf> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(default.to_path_buf());
    }
    // Expand a leading `~` to the home directory.
    if let Some(rest) = trimmed.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return Ok(home.join(rest));
    }
    Ok(PathBuf::from(trimmed))
}

fn prompt_bool(prompt: &str) -> Result<bool> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(!matches!(input.trim().to_lowercase().as_str(), "n" | "no"))
}
