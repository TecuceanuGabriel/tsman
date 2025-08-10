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

pub fn get_session() -> Result<Session> {
    let (name, path) =
        get_session_info().context("Failed to get session info")?;
    let windows = get_windows().context("Failed to get windows")?;

    Ok(Session {
        name,
        work_dir: path,
        windows,
    })
}

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

pub fn close_session(session_name: &str) -> Result<()> {
    Command::new("tmux")
        .arg("kill-session")
        .args(["-t", session_name])
        .status()
        .context("Failed to kill session")?;

    Ok(())
}

pub fn get_session_info() -> Result<(String, String)> {
    let output = Command::new("tmux")
        .arg("display-message")
        .arg("-p")
        .args(["-F", "#{session_name} #{session_path}"])
        .output()
        .context("Failed to execute 'tmux display-message'")?;

    let string_output = String::from_utf8(output.stdout)
        .context("Failed to convert tmux output to UTF-8 string")?;

    let mut parts = string_output.trim().split(" ");

    match (parts.next(), parts.next()) {
        (Some(session_name), Some(session_path)) => {
            Ok((session_name.to_string(), session_path.to_string()))
        }
        _ => anyhow::bail!(
            "Failed to parse session name and path from: '{}'",
            string_output
        ),
    }
}

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

fn get_windows() -> Result<Vec<Window>> {
    let output = Command::new("tmux")
        .arg("list-windows")
        .args(["-F", "#{window_index} #{window_name} #{window_layout}"])
        .output()
        .context("Failed to execute 'tmux list-windows'")?;

    let string_output = String::from_utf8(output.stdout)
        .context("Failed to convert tmux output to UTF-8 string")?;

    string_output
        .trim()
        .split(TMUX_LINE_SEPARATOR)
        .map(parse_window_string)
        .collect()
}

fn parse_window_string(window: &str) -> Result<Window> {
    let mut parts = window.split(" ");

    match (parts.next(), parts.next(), parts.next()) {
        (Some(index), Some(name), Some(layout)) => {
            let index = index.to_string();
            let panes = get_panes(&index)?;

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

fn get_panes(window_id: &str) -> Result<Vec<Pane>> {
    let output = Command::new("tmux")
        .arg("list-panes")
        .args(["-t", window_id])
        .args(["-F", "#{pane_index} #{pane_pid} #{pane_current_path}"])
        .output()
        .with_context(|| {
            format!(
                "Failed to execute 'tmux list-panes' for window {window_id}",
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
        if let Some((pid_str, cmdline)) = trimmed.split_once(' ') {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                children.push((pid, cmdline.trim().to_string()));
            }
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
