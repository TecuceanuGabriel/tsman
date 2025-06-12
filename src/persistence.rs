use dirs::home_dir;
use std::fs;
use std::path::PathBuf;

use crate::tmux_interface::*;

const CONFIG_DIR: &str = ".tsessions";

fn get_config_file_path(file_name: &str) -> Result<PathBuf, TmuxError> {
    let mut path = home_dir().ok_or(TmuxError::InvalidOutputFormat(
        "Could not determine Home Dir".to_string(), // TODO: use appropriate error type
    ))?;
    path.push(".config");
    path.push(CONFIG_DIR);
    path.push(format!("{}.yaml", file_name));
    Ok(path)
}

pub fn save_session_config(
    file_name: &str,
    data: String,
) -> Result<(), TmuxError> {
    let path = get_config_file_path(file_name)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, data)?;
    Ok(())
}

pub fn load_session_from_config(file_name: &str) -> Result<Session, TmuxError> {
    todo!();
}

pub fn list_saved_sessions() -> Result<Vec<String>, TmuxError> {
    todo!();
}
