use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Pane {
    id: String,
    current_command: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Window {
    id: String,
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

fn parse_pane_string(pane: &str) -> Result<Pane> {
    let mut parts = pane.split(TMUX_FIELD_SEPARATOR);

    match (parts.next(), parts.next()) {
        (Some(id), Some(current_command)) => Ok(Pane {
            id: id.to_string(),
            current_command: current_command.to_string(),
        }),
        _ => anyhow::bail!("Failed to parse pane string: {}", pane),
    }
}

fn get_panes(window_id: &str) -> Result<Vec<Pane>> {
    let output = Command::new("tmux")
        .arg("list-panes")
        .args(["-t", window_id])
        .args(["-F", "#{pane_id} #{pane_current_command}"])
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
        (Some(id), Some(name), Some(layout)) => {
            let id = id.to_string();
            let panes = get_panes(&id)?;

            Ok(Window {
                id,
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
        .args(["-F", "#{window_id} #{window_name} #{window_layout}"])
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

pub fn restore_session(session: Session) -> Result<()> {
    todo!();
}
