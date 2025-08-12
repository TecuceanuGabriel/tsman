use std::borrow::Cow;
use std::env;
use std::fs::write;
use std::process::Command;

use anyhow::{Context, Result};
use shell_escape::escape;
use tempfile::NamedTempFile;

use crate::tmux::session::*;

const TMUX_FIELD_SEPARATOR: &str = " ";
const TMUX_LINE_SEPARATOR: &str = "\n";

/// Retrives a [`Session`] by name, or infer the current session if a name is
/// not provided.
///
/// # Arguments
///
/// *  `session_name` - name of the tmux session to retrive (optional). If
/// `None`, uses [`get_session_name`] to detect the current session.
///
/// # Returns
///
/// A fully populated [`Session`] struct.
///
/// # Errors
///
/// Returns an error if:
/// - The session cannot be determined/there is no attached session.
/// - Any tmux command used to gather details fails
pub fn get_session(session_name: Option<&str>) -> Result<Session> {
    let name = if let Some(name) = session_name {
        name.to_string()
    } else {
        get_session_name()?
    };

    let path = get_session_path(&name)?;

    let windows = get_windows(&name).context("Failed to get windows")?;

    Ok(Session {
        name,
        work_dir: path,
        windows,
    })
}

/// Restores a tmux session from a [`Session`] struct.
///
/// Creates a temporary session, populates it with windows and panes, then 
/// renames it to the target name to avoid naming conflicts.
///
/// # Arguments
/// * `session` – The [`Session`] to restore.
///
/// # Process
/// 1. Create a temporary session.
/// 2. Create windows:
///     - Create panes
///     - Restore layout
///     - Change into work dir and run commands
/// 3. Rename the temporary session to the target name.
/// 4. Attach to the restored session.
///
/// # Errors
/// Returns an error if any tmux command fails, or if writing the temporary 
/// restoration script fails.
pub fn restore_session(session: &Session) -> Result<()> {
    let temp_session_name = format!("tsman-temp-{}", std::process::id());

    let mut script_str = String::new();

    script_str += &format!(
        "tmux new-session -d -s {} -c {}\n",
        temp_session_name,
        escape(Cow::from(&session.work_dir))
    );

    let first_window = &session.windows[0];

    script_str +=
        &get_window_config_cmd(&temp_session_name, session, first_window)?;

    for window in session.windows.iter().skip(1) {
        script_str += &format!(
            "tmux new-window -d -t {} -n {} -c {}\n",
            temp_session_name,
            window.name,
            escape(Cow::from(&session.work_dir))
        );

        script_str +=
            &get_window_config_cmd(&temp_session_name, session, window)?;
    }

    // this helps avoid naming conflicts inside tmux
    script_str += &format!(
        "tmux rename-session -t {} {}\n",
        temp_session_name, session.name
    );

    let script = NamedTempFile::new()?;

    write(script.path(), script_str)?;

    Command::new("sh")
        .arg(script.path())
        .status()
        .context("Failed to reconstruct session")?;

    attach_to_session(&session.name)
}

/// Checks if a tmux session is currently active.
///
/// # Arguments
/// * `session_name` – The name of the tmux session.
///
/// # Returns
/// `Ok(true)` if the session exists, `Ok(false)` otherwise.
///
/// # Errors
/// Returns an error if the `tmux list-session` command fails.
pub fn is_active_session(session_name: &str) -> Result<bool> {
    let output = Command::new("tmux")
        .arg("list-session")
        .args(["-F", "#{session_name}"])
        .output()
        .context("Failed to get sessions")?;

    let output_str = String::from_utf8(output.stdout)?;
    let session_names =
        output_str.split(TMUX_LINE_SEPARATOR).collect::<Vec<&str>>();

    Ok(session_names.contains(&session_name))
}

/// Attaches to or switches to a tmux session.
///
/// If already inside tmux, uses `switch-client`.  
/// If outside, uses `attach-session`.
///
/// # Arguments
/// * `session_name` – The session name to attach to.
///
/// # Errors
/// Returns an error if the tmux attach/switch command fails.
pub fn attach_to_session(session_name: &str) -> Result<()> {
    let is_attached = env::var("TMUX").is_ok();
    let attach_cmd = if is_attached {
        "switch-client"
    } else {
        "attach-session"
    };

    Command::new("tmux")
        .arg(attach_cmd)
        .args(["-t", session_name])
        .status()
        .context("Failed to attach session")?;

    Ok(())
}

/// Closes a tmux session by name.
///
/// # Arguments
/// * `session_name` – The session to kill.
///
/// # Errors
/// Returns an error if `tmux kill-session` fails.
pub fn close_session(session_name: &str) -> Result<()> {
    Command::new("tmux")
        .arg("kill-session")
        .args(["-t", session_name])
        .status()
        .context("Failed to kill session")?;

    Ok(())
}

/// Gets the name of the current tmux session.
///
/// # Returns
/// The current session name as a `String`.
///
/// # Errors
/// Returns an error if tmux fails to execute or output parsing fails.
pub fn get_session_name() -> Result<String> {
    let output = Command::new("tmux")
        .arg("display-message")
        .arg("-p")
        .args(["-F", "#{session_name}"])
        .output()
        .context("Failed to execute 'tmux display-message'")?;

    let string_output = String::from_utf8(output.stdout)
        .context("Failed to convert tmux output to UTF-8 string")?;

    Ok(string_output.trim().to_string())
}

/// Lists all currently active tmux sessions.
///
/// # Returns
/// A vector of session names.
///
/// # Behavior
/// If the tmux server is not running, returns an empty vector.
///
/// # Errors
/// Returns an error if tmux commands fail.
pub fn list_active_sessions() -> Result<Vec<String>> {
    let status = Command::new("tmux")
        .arg("has-session")
        .status()
        .context("Failed to check tmux server status")?;

    if !status.success() {
        return Ok(Vec::new()); // server not running
    }

    let output = Command::new("tmux")
        .arg("list-sessions")
        .args(["-F", "#{session_name}"])
        .output()
        .context("Failed to get active sessions")?;

    let string_output = String::from_utf8(output.stdout)
        .context("Failed to convert tmux output to UTF-8 string")?;

    let parts: Vec<String> = string_output
        .trim()
        .split(TMUX_LINE_SEPARATOR)
        .map(|s| s.to_string())
        .collect();

    Ok(parts)
}

/// Retrieves the working directory path of a tmux session.
///
/// # Arguments
/// * `session_name` – Name of the tmux session.
///
/// # Errors
/// Returns an error if tmux command execution or parsing fails.
fn get_session_path(session_name: &str) -> Result<String> {
    let output = Command::new("tmux")
        .arg("display-message")
        .arg("-p")
        .args(["-t", session_name])
        .args(["-F", "#{session_path}"])
        .output()
        .context("Failed to execute 'tmux display-message'")?;

    let string_output = String::from_utf8(output.stdout)
        .context("Failed to convert tmux output to UTF-8 string")?;

    Ok(string_output.trim().to_string())
}

/// Retrieves all windows of a tmux session.
///
/// # Arguments
/// * `session_name` – The tmux session name.
///
/// # Returns
/// A vector of [`Window`] structs.
///
/// # Errors
/// Returns an error if `tmux list-windows` fails or parsing fails.
fn get_windows(session_name: &str) -> Result<Vec<Window>> {
    let output = Command::new("tmux")
        .arg("list-windows")
        .args(["-t", session_name])
        .args(["-F", "#{window_index} #{window_name} #{window_layout}"])
        .output()
        .context("Failed to execute 'tmux list-windows'")?;

    let string_output = String::from_utf8(output.stdout)
        .context("Failed to convert tmux output to UTF-8 string")?;

    string_output
        .trim()
        .split(TMUX_LINE_SEPARATOR)
        .map(|window| parse_window_string(window, session_name))
        .collect()
}

/// Parses a single tmux window info string into a [`Window`] struct.
///
/// # Format
/// `"INDEX NAME LAYOUT"`
///
/// # Errors
/// Returns an error if the format is invalid or if panes cannot be retrieved.
fn parse_window_string(window: &str, session_name: &str) -> Result<Window> {
    let mut parts = window.split(" ");

    match (parts.next(), parts.next(), parts.next()) {
        (Some(index), Some(name), Some(layout)) => {
            let index = index.to_string();
            let window_target = format!("{session_name}:{index}");
            let panes = get_panes(&window_target)?;

            Ok(Window {
                index,
                name: name.to_string(),
                layout: layout.to_string(),
                panes,
            })
        }
        _ => {
            anyhow::bail!(format!("Failed to parse window string: {}", window))
        }
    }
}

/// Retrieves all panes for a given tmux window.
///
/// # Arguments
/// * `window_target` – Format: `"SESSION:WINDOW_INDEX"`.
///
/// # Returns
/// A vector of [`Pane`] structs.
///
/// # Errors
/// Returns an error if tmux fails or parsing fails.
fn get_panes(window_target: &str) -> Result<Vec<Pane>> {
    let output = Command::new("tmux")
        .arg("list-panes")
        .args(["-t", window_target])
        .args(["-F", "#{pane_index} #{pane_pid} #{pane_current_path}"])
        .output()
        .with_context(|| {
            format!(
                "Failed to execute 'tmux list-panes' for window {window_target}",
            )
        })?;

    let string_output = String::from_utf8(output.stdout)
        .context("Failed to convert tmux output to UTF-8 string")?;

    string_output
        .trim()
        .split(TMUX_LINE_SEPARATOR)
        .map(parse_pane_string)
        .collect()
}

/// Parses a pane information string into a [`Pane`] struct.
///
/// # Format
/// `"INDEX PID WORK_DIR"`
///
/// # Behavior
/// Attempts to detect the currently running foreground process inside the pane.
///
/// # Errors
/// Returns an error if parsing fails or process lookup fails.
fn parse_pane_string(pane: &str) -> Result<Pane> {
    let mut parts = pane.split(TMUX_FIELD_SEPARATOR);

    match (parts.next(), parts.next(), parts.next()) {
        (Some(index), Some(pid), Some(work_dir_str)) => {
            let process = get_foreground_process(pid)?;

            let current_command = match process {
                Some((cmd_pid, cmdline)) if std::process::id() != cmd_pid => {
                    Some(cmdline)
                }
                _ => None,
            };

            Ok(Pane {
                index: index.to_string(),
                current_command,
                work_dir: work_dir_str.to_string(),
            })
        }
        _ => anyhow::bail!("Failed to parse pane string: {}", pane),
    }
}

/// Retrieves the first child process of a shell process.
///
/// # Arguments
/// * `shell_pid` – PID of the shell process.
///
/// # Returns
/// The PID and command line of the first child process, if any.
fn get_foreground_process(shell_pid: &str) -> Result<Option<(u32, String)>> {
    Ok(get_process_children(shell_pid)?.into_iter().next())
}

/// Lists the immediate child processes of a given PID.
///
/// # Arguments
/// * `shell_pid` – Parent process PID.
///
/// # Returns
/// A vector of `(PID, command_line)` tuples.
///
/// # Errors
/// Returns an error if the `ps` command fails or parsing fails.
fn get_process_children(shell_pid: &str) -> Result<Vec<(u32, String)>> {
    let output = Command::new("ps")
        .args(["-o", "pid=,args="])
        .args(["--ppid", shell_pid])
        .output()
        .with_context(|| {
            format!("Failed to get children of process #{shell_pid}")
        })?;

    let output_str = String::from_utf8(output.stdout)?;

    let mut children = Vec::new();

    for line in output_str.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((pid_str, cmdline)) = trimmed.split_once(' ')
            && let Ok(pid) = pid_str.trim().parse::<u32>()
        {
            children.push((pid, cmdline.trim().to_string()));
        }
    }

    Ok(children)
}

/// Builds tmux commands to configure a window's panes, layout, and commands.
///
/// # Arguments
/// * `temp_session_name` – Temporary session name during restore.
/// * `session` – Full session data.
/// * `window` – Window data to restore.
///
/// # Returns
/// A string containing tmux commands.
///
/// # Errors
/// Returns an error if escaping paths or commands fails.
fn get_window_config_cmd(
    temp_session_name: &str,
    session: &Session,
    window: &Window,
) -> Result<String> {
    let window_target = format!("{}:{}", temp_session_name, window.index);

    let mut cmd = String::new();

    for _ in window.panes.iter().skip(1) {
        cmd += &format!(
            "tmux split-window -d -t {} -c {}\n",
            window_target,
            escape(Cow::from(&session.work_dir))
        );
    }

    cmd += &format!(
        "tmux select-layout -t {} {}\n",
        window_target,
        escape(Cow::from(&window.layout))
    );

    for pane in &window.panes {
        let pane_target = format!("{}.{}", window_target, pane.index);

        if pane.work_dir != session.work_dir {
            cmd += &format!(
                "tmux send-keys -t {} {} C-m\n",
                pane_target,
                escape(
                    format!("cd {}; clear", escape(Cow::from(&pane.work_dir)))
                        .into()
                ),
            );
        }

        if let Some(pane_cmd) = &pane.current_command {
            cmd += &format!(
                "tmux send-keys -t {} {} C-m\n",
                pane_target,
                escape(pane_cmd.into())
            );
        }
    }

    Ok(cmd)
}
