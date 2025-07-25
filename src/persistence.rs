use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use dirs::home_dir;

const CONFIG_DIR: &str = ".tsessions";

pub fn save_session_config(file_name: &str, data: String) -> Result<()> {
    let path = get_config_file_path(file_name)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create directory {}", parent.display())
        })?;
    }

    fs::write(&path, data)?;
    Ok(())
}

pub fn load_session_from_config(file_name: &str) -> Result<String> {
    let path = get_config_file_path(file_name)?;
    let data = fs::read_to_string(path)?;
    Ok(data)
}

pub fn list_saved_sessions() -> Result<Vec<String>> {
    todo!();
}

fn get_config_file_path(file_name: &str) -> Result<PathBuf> {
    let mut path = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine HOME directory"))?;

    path.push(".config");
    path.push(CONFIG_DIR);
    path.push(format!("{}.yaml", file_name));
    Ok(path)
}
