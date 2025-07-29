use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use dirs::home_dir;

const CONFIG_DIR: &str = ".tsessions"; // TODO: make this configurable

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
    let dir_path = get_default_session_dir_path()?;

    let paths = fs::read_dir(dir_path.into_os_string())?;
    let mut result = Vec::with_capacity(paths.size_hint().0);

    for entry in paths {
        let path = entry?.path();

        let name = path
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!("Missing file stem for {:?}", path))?
            .to_str()
            .ok_or_else(|| {
                anyhow::anyhow!("Invalid UTF-8 filename: {:?}", path)
            })?;

        result.push(name.to_owned());
    }

    Ok(result)
}

pub fn get_config_file_path(file_name: &str) -> Result<PathBuf> {
    let mut path = get_default_session_dir_path()?;
    path.push(format!("{}.yaml", file_name));
    Ok(path)
}

fn get_default_session_dir_path() -> Result<PathBuf> {
    let mut path = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine HOME directory"))?;

    path.push(".config");
    path.push(CONFIG_DIR);

    Ok(path)
}
