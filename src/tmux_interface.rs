use serde::{Deserialize, Serialize};
use std::{io, process::Command};

#[derive(Debug)]
pub enum TmuxError {
    CommandExecution(io::Error),
    InvalidUtf8(std::string::FromUtf8Error),
    InvalidOutputFormat(String),
}

impl From<io::Error> for TmuxError {
    fn from(err: io::Error) -> Self {
        TmuxError::CommandExecution(err)
    }
}

impl From<std::string::FromUtf8Error> for TmuxError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        TmuxError::InvalidUtf8(err)
    }
}

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

fn parse_pane_string(pane: &str) -> Result<Pane, TmuxError> {
    let mut parts = pane.split(" ");

    match (parts.next(), parts.next()) {
        (Some(id), Some(current_command)) => Ok(Pane {
            id: id.to_string(),
            current_command: current_command.to_string(),
        }),
        _ => Err(TmuxError::InvalidOutputFormat(
            "failed to parse pane string".to_string(),
        )),
    }
}

fn get_panes(window_id: &str) -> Result<Vec<Pane>, TmuxError> {
    let output = Command::new("tmux")
        .arg("list-panes")
        .args(["-t", window_id])
        .args(["-F", "#{pane_id} #{pane_current_command}"])
        .output()?;

    let string_output = String::from_utf8(output.stdout)?;

    let lines = string_output.trim().split("\n");

    let mut panes = Vec::new();
    for line in lines {
        let pane = parse_pane_string(line)?;
        panes.push(pane);
    }

    Ok(panes)
}

fn parse_window_string(window: &str) -> Result<Window, TmuxError> {
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
        _ => Err(TmuxError::InvalidOutputFormat(
            "failed to parse window string".to_string(),
        )),
    }
}

fn get_windows() -> Result<Vec<Window>, TmuxError> {
    let output = Command::new("tmux")
        .arg("list-windows")
        .args(["-F", "#{window_id} #{window_name} #{window_layout}"])
        .output()?;

    let string_output = String::from_utf8(output.stdout)?;

    let lines = string_output.trim().split("\n");

    let mut windows = Vec::new();
    for line in lines {
        let window = parse_window_string(line)?;
        windows.push(window);
    }

    Ok(windows)
}

fn get_session_info() -> Result<(String, String), TmuxError> {
    let output = Command::new("tmux")
        .arg("display-message")
        .arg("-p")
        .args(["-F", "#{session_name} #{session_path}"])
        .output()?;

    let string_output = String::from_utf8(output.stdout)?;

    let mut parts = string_output.trim().split(" "); // TODO: const for separator?

    match (parts.next(), parts.next()) {
        (Some(session_name), Some(session_path)) => {
            Ok((session_name.to_string(), session_path.to_string()))
        }
        _ => Err(TmuxError::InvalidOutputFormat(
            "failed to parse session name and path".to_string(),
        )),
    }
}

pub fn get_session() -> Result<Session, TmuxError> {
    let (name, path) = get_session_info()?;
    let windows = get_windows()?;

    Ok(Session {
        name,
        path,
        windows,
    })
}

pub fn restore_session(session: Session) -> Result<(), TmuxError> {
    todo!();
}
