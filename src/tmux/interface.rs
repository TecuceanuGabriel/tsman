//! Tmux interface - all tmux interaction goes through [`std::process::Command`].
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

/// Captures a [`Session`] by name, or the currently attached session if `None`.
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

/// Restores a [`Session`] by generating a shell script that creates a temp
/// session, configures windows/panes, then renames it to avoid conflicts.
pub fn restore_session(session: &Session) -> Result<()> {
    let temp_name = format!("tsman-temp-{}", std::process::id());
    create_session_from_config(session, &temp_name)?;
    rename_session(&temp_name, &session.name)?;
    attach_to_session(&session.name)
}

/// Kills a running session and recreates it from the saved config.
///
/// Creates a new session under a temp name, switches the client to it,
/// kills the old session, then renames. This avoids tmux closing when
/// reloading the currently attached session.
pub fn reload_session(session: &Session) -> Result<()> {
    let temp_name = format!("tsman-temp-{}", std::process::id());
    create_session_from_config(session, &temp_name)?;
    attach_to_session(&temp_name)?;
    close_session(&session.name)?;
    rename_session(&temp_name, &session.name)?;
    Ok(())
}

/// Creates a tmux session from config under the given name, without
/// attaching or renaming.
fn create_session_from_config(
    session: &Session,
    session_name: &str,
) -> Result<()> {
    let mut script_str = String::new();

    script_str += &format!(
        "tmux new-session -d -s {} -c {}\n",
        session_name,
        escape(Cow::from(&session.work_dir))
    );

    let first_window = &session.windows[0];

    script_str += &get_window_config_cmd(session_name, session, first_window)?;

    for window in session.windows.iter().skip(1) {
        script_str += &format!(
            "tmux new-window -d -t {} -c {}\n",
            session_name,
            escape(Cow::from(&session.work_dir))
        );

        script_str += &get_window_config_cmd(session_name, session, window)?;
    }

    let script = NamedTempFile::new()?;

    write(script.path(), script_str)?;

    Command::new("sh")
        .arg(script.path())
        .status()
        .context("Failed to reconstruct session")?;

    Ok(())
}

/// Returns whether a tmux session with the given name exists.
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

/// Attaches to a session. Uses `switch-client` if inside tmux, `attach-session` otherwise.
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

/// Renames an active tmux session.
pub fn rename_session(session_name: &str, new_name: &str) -> Result<()> {
    Command::new("tmux")
        .arg("rename-session")
        .args(["-t", session_name])
        .arg(new_name)
        .status()
        .context("Failed to rename session")?;

    Ok(())
}

/// Kills a tmux session by name.
///
/// If the session being killed is the one we are currently attached to,
/// switches to the next active session first so tmux doesn't close the
/// client. If there is no other session, the kill proceeds normally
/// (tmux will detach).
pub fn close_session(session_name: &str) -> Result<()> {
    if let Ok(current) = get_session_name()
        && current == session_name
        && let Some(next) = get_next_session(session_name)?
    {
        attach_to_session(&next)?;
    }

    Command::new("tmux")
        .arg("kill-session")
        .args(["-t", session_name])
        .status()
        .context("Failed to kill session")?;

    Ok(())
}

/// Returns the next active session after `session_name` in the session list,
/// or `None` if there are no other sessions.
fn get_next_session(session_name: &str) -> Result<Option<String>> {
    let sessions = list_active_sessions()?;
    let pos = sessions.iter().position(|s| s == session_name).unwrap_or(0);

    // Walk forward from the current position, wrapping around.
    for i in 1..sessions.len() {
        let candidate = &sessions[(pos + i) % sessions.len()];
        if candidate != session_name {
            return Ok(Some(candidate.clone()));
        }
    }

    Ok(None)
}

/// Returns the name of the currently attached tmux session.
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

/// Lists all active tmux session names. Returns an empty vec if the server is not running.
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

fn get_foreground_process(shell_pid: &str) -> Result<Option<(u32, String)>> {
    Ok(get_process_children(shell_pid)?.into_iter().next())
}

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

fn get_window_config_cmd(
    temp_session_name: &str,
    session: &Session,
    window: &Window,
) -> Result<String> {
    let window_target = format!("{}:{}", temp_session_name, window.index);

    let mut cmd = String::new();

    cmd +=
        &format!("tmux rename-window -t {} {}\n", window_target, window.name);

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
