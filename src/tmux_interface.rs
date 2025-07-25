use std::process::Command;
use std::thread::sleep_ms;
use std::{fs::write, thread::sleep};
use tempfile::NamedTempFile;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pane {
    index: String,
    current_command: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Window {
    index: String,
    name: String,
    layout: String,
    panes: Vec<Pane>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Session {
    pub name: String,
    path: String,
    windows: Vec<Window>,
}

const TMUX_FIELD_SEPARATOR: &str = " ";
const TMUX_LINE_SEPARATOR: &str = "\n";

fn get_process_children(pid: u32) -> Result<Vec<String>> {
    let output = Command::new("ps")
        .args(["-o", "args="])
        .args(["--ppid", &pid.to_string()])
        .output()
        .with_context(|| {
            format!("Failed to get children of process #{}", pid)
        })?;

    let mut children = Vec::new();
    let output_str = String::from_utf8(output.stdout)?;

    for line in output_str.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            children.push(trimmed.to_string());
        }
    }

    Ok(children)
}

fn get_foreground_process(shell_pid: u32) -> Result<Option<String>> {
    match get_process_children(shell_pid) {
        Ok(children) => Ok(children.first().cloned()),
        _ => Ok(None),
    }
}

fn parse_pane_string(pane: &str) -> Result<Pane> {
    let mut parts = pane.split(TMUX_FIELD_SEPARATOR);

    match (parts.next(), parts.next()) {
        (Some(index), Some(pid_str)) => {
            let pid = pid_str.parse::<u32>()?;

            let current_command = get_foreground_process(pid)?;

            Ok(Pane {
                index: index.to_string(),
                current_command,
            })
        }
        _ => anyhow::bail!("Failed to parse pane string: {}", pane),
    }
}

fn get_panes(window_id: &str) -> Result<Vec<Pane>> {
    let output = Command::new("tmux")
        .arg("list-panes")
        .args(["-t", window_id])
        .args(["-F", "#{pane_index} #{pane_pid}"])
        .output()
        .with_context(|| {
            format!(
                "Failed to execute 'tmux list-panes' for window {}",
                window_id
            )
        })?;

    let string_output = String::from_utf8(output.stdout)
        .context("Failed to convert tmux output to UTF-8 string")?;

    string_output
        .trim()
        .split(TMUX_LINE_SEPARATOR)
        .map(|line| parse_pane_string(line))
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
        .map(|line| parse_window_string(line))
        .collect()
}

fn get_session_info() -> Result<(String, String)> {
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

pub fn get_session() -> Result<Session> {
    let (name, path) =
        get_session_info().context("Failed to get session info")?;
    let windows = get_windows().context("Failed to get windows")?;

    Ok(Session {
        name,
        path,
        windows,
    })
}

fn get_window_config_cmd(session_name: &str, window: Window) -> Result<String> {
    let window_target = format!("{}:{}", session_name, window.index);

    let mut cmd = String::new();

    for _ in window.panes.iter().skip(1) {
        cmd += &format!("tmux split-window -d -t {}\n", window_target);
    }

    cmd += &format!(
        "tmux select-layout -t {} \"{}\"\n",
        window_target, window.layout
    );

    for pane in window.panes {
        let pane_target = format!("{}.{}", window_target, pane.index);

        if let Some(pane_cmd) = pane.current_command {
            cmd += &format!(
                "tmux send-keys -t {} \"{}\" C-m\n",
                pane_target, pane_cmd
            );
        }
    }

    Ok(cmd)
}

pub fn restore_session(session: Session) -> Result<()> {
    let mut script_str = String::new();

    script_str += &format!(
        "tmux new-session -d -s {} -c {}\n",
        session.name, session.path
    );

    let first_window = session.windows[0].clone();

    script_str += &get_window_config_cmd(&session.name, first_window).unwrap();

    for window in session.windows.into_iter().skip(1) {
        script_str += &format!(
            "tmux new-window -d -t {} -n {}\n",
            session.name, window.name
        );

        script_str += &get_window_config_cmd(&session.name, window).unwrap();
    }

    let script = NamedTempFile::new().unwrap();

    write(script.path(), script_str)?;

    Command::new("sh")
        .arg(script.path())
        .status()
        .context("Failed to reconstruct session")?;

    sleep_ms(700);

    Command::new("tmux")
        .arg("attach-session")
        .args(["-t", &session.name])
        .status()
        .context("Failed to attach session")?;

    Ok(())
}
