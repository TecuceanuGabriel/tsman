use std::borrow::Cow;
use std::env;
use std::fs::write;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use shell_escape::escape;
use tempfile::NamedTempFile;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pane {
    index: String,
    current_command: Option<String>,
    work_dir: String,
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
    work_dir: String,
    windows: Vec<Window>,
}

const TMUX_FIELD_SEPARATOR: &str = " ";
const TMUX_LINE_SEPARATOR: &str = "\n";

const ATTACH_DELAY: u64 = 700;

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
    let output = Command::new("tmux")
        .arg("list-session")
        .args(["-F", "#{session_name}"])
        .output()
        .context("Failed to get sessions")?;

    let output_str = String::from_utf8(output.stdout)?;
    let session_names =
        output_str.split(TMUX_LINE_SEPARATOR).collect::<Vec<&str>>();

    if !session_names.contains(&session.name.as_str()) {
        let mut script_str = String::new();

        script_str += &format!(
            "tmux new-session -d -s {} -c {}\n",
            session.name,
            escape(Cow::from(&session.work_dir))
        );

        let first_window = &session.windows[0];

        script_str += &get_window_config_cmd(&session, &first_window)?;

        for window in session.windows.iter().skip(1) {
            script_str += &format!(
                "tmux new-window -d -t {} -n {}\n",
                session.name, window.name
            );

            script_str += &get_window_config_cmd(session, &window)?;
        }

        let script = NamedTempFile::new()?;

        write(script.path(), script_str)?;

        Command::new("sh")
            .arg(script.path())
            .status()
            .context("Failed to reconstruct session")?;

        sleep(Duration::from_millis(ATTACH_DELAY));
    }

    let is_attached = match env::var("TMUX") {
        Ok(s) => !s.is_empty(),
        _ => false,
    };

    if is_attached {
        Command::new("tmux")
            .arg("switch-client")
            .args(["-t", &session.name])
            .status()
            .context("Failed to attach session")?;
    } else {
        Command::new("tmux")
            .arg("attach-session")
            .args(["-t", &session.name])
            .status()
            .context("Failed to attach session")?;
    }

    Ok(())
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
            format!("Failed to get children of process #{}", shell_pid)
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

fn get_window_config_cmd(session: &Session, window: &Window) -> Result<String> {
    let window_target = format!("{}:{}", session.name, window.index);

    let mut cmd = String::new();

    for _ in window.panes.iter().skip(1) {
        cmd += &format!("tmux split-window -d -t {}\n", window_target);
    }

    cmd += &format!(
        "tmux select-layout -t {} \"{}\"\n",
        window_target, window.layout
    );

    for pane in &window.panes {
        let pane_target = format!("{}.{}", window_target, pane.index);

        if let Some(pane_cmd) = &pane.current_command {
            if pane.work_dir != session.work_dir {
                cmd += &format!(
                    "tmux send-keys -t {} {} C-m\n",
                    pane_target,
                    escape(format!("cd {}", pane.work_dir).into()),
                )
            }
            cmd += &format!(
                "tmux send-keys -t {} {} C-m\n",
                pane_target,
                escape(pane_cmd.into())
            );
        }
    }

    Ok(cmd)
}
