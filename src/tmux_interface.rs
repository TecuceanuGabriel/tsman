use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Pane {
    index: String,
    current_command: String,
}

#[derive(Serialize, Deserialize, Debug)]
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

fn parse_pane_string(pane: &str) -> Result<Pane> {
    let mut parts = pane.split(TMUX_FIELD_SEPARATOR);

    match (parts.next(), parts.next()) {
        (Some(index), Some(current_command)) => Ok(Pane {
            index: index.to_string(),
            current_command: current_command.to_string(),
        }),
        _ => anyhow::bail!("Failed to parse pane string: {}", pane),
    }
}

fn get_panes(window_id: &str) -> Result<Vec<Pane>> {
    let output = Command::new("tmux")
        .arg("list-panes")
        .args(["-t", window_id])
        .args(["-F", "#{pane_index} #{pane_current_command}"])
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

fn configure_window(session_name: &str, window: &Window) -> Result<()> {
    let window_target = format!("{}:{}", session_name, window.index);

    for _ in window.panes.iter().skip(1) {
        Command::new("tmux")
            .arg("split-window")
            .arg("-d")
            .args(["-t", &window_target])
            .status()
            .context("Failed to execute 'tmux split-window'")?;
    }

    Command::new("tmux")
        .arg("select-layout")
        .args(["-t", &window_target, &window.layout])
        .status()
        .context("Failed to execute 'tmux select-layout'")?;

    for pane in window.panes.iter() {
        // println!("{} : command {}", pane.index, pane.current_command);

        Command::new("tmux")
            .arg("send-keys")
            .args(["-t", &format!("{}.{}", window_target, pane.index)])
            .args([&pane.current_command, "C-m"])
            .status()
            .context("Failed to send command to pane")?;
    }

    Ok(())
}

pub fn restore_session(session: Session) -> Result<()> {
    // TODO: check if session already exists

    Command::new("tmux")
        .arg("new-session")
        .arg("-d")
        .args(["-s", &session.name])
        .args(["-c", &session.path])
        .status()
        .context("Failed to execute 'tmux new-session'")?;

    let first_window = &session.windows[0];
    configure_window(&session.name, first_window)?;

    for window in session.windows.iter().skip(1) {
        Command::new("tmux")
            .arg("new-window")
            .arg("-d")
            .args(["-t", &format!("{}:", session.name)])
            .args(["-n", &window.name])
            .status()
            .with_context(|| {
                format!("Failed to create window '{}'", window.name)
            })?;

        configure_window(&session.name, window)?;
    }

    Command::new("tmux")
        .arg("attach-session")
        .args(["-t", &session.name])
        .status()
        .context("Failed to execute 'tmux attach-session'")?;

    Ok(())
}
