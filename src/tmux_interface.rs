use std::process::Command;
use std::sync::mpsc;
use std::thread;

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
        let mut parts = line.trim().split(' ');
        if let Some(args) = parts.next() {
            children.push(args.to_string());
        } else {
            anyhow::bail!("Failed to parse process children: #{}", pid);
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

fn configure_window(session_name: &str, window: Window) -> Result<()> {
    let window_target = format!("{}:{}", session_name, window.index);

    for _ in window.panes.iter().skip(1) {
        Command::new("tmux")
            .arg("split-window")
            .arg("-d")
            .args(["-t", &window_target])
            .status()
            .context("Failed to split window")?;
    }

    Command::new("tmux")
        .arg("select-layout")
        .args(["-t", &window_target, &window.layout])
        .status()
        .context("Failed to set layout")?;

    let (tx, rx) = mpsc::channel();

    let mut handles = vec![];

    for pane in window.panes {
        let tx = tx.clone();
        let pane_cmd = pane.current_command;
        let pane_target = format!("{}.{}", window_target, pane.index);

        let handle = thread::spawn(move || {
            let result = Command::new("tmux")
                .arg("send-keys")
                .args(["-t", &pane_target])
                .args([&pane_cmd, "C-m"])
                .status()
                .context(format!(
                    "Failed to send command to pane {}",
                    pane.index
                ));

            tx.send(result).unwrap();
        });

        handles.push(handle)
    }

    drop(tx);
    for result in rx {
        result?;
    }

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}

pub fn restore_session(session: Session) -> Result<()> {
    Command::new("tmux")
        .arg("new-session")
        .arg("-d")
        .args(["-s", &session.name])
        .args(["-c", &session.path])
        .status()
        .context("Failed to execute 'tmux new-session'")?;

    let first_window = session.windows[0].clone();
    configure_window(&session.name, first_window)?;

    let session_name = session.name.clone();
    let windows = session.windows.clone();

    let (tx, rx) = mpsc::channel();
    let mut handles = vec![];

    for window in windows.into_iter().skip(1) {
        let tx = tx.clone();
        let session_name = session_name.clone();

        let handle = thread::spawn(move || {
            let result = (|| {
                Command::new("tmux")
                    .arg("new-window")
                    .arg("-d")
                    .args(["-t", &format!("{}:", session_name)])
                    .args(["-n", &window.name])
                    .status()
                    .context("Failed to create window")?;

                configure_window(&session_name, window)
            })();

            tx.send(result).unwrap();
        });

        handles.push(handle);
    }

    drop(tx);
    for result in rx {
        result.context("Window creation failed")?;
    }

    for handle in handles {
        handle.join().unwrap();
    }

    Command::new("tmux")
        .arg("attach-session")
        .args(["-t", &session.name])
        .status()
        .context("Failed to attach session")?;

    Ok(())
}
