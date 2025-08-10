use std::path::PathBuf;
use std::{env, fs};

use anyhow::{Context, Result};
use dirs::home_dir;

const DEFAULT_CONFIG_STORAGE_DIR: &str = ".tsessions";

pub fn save_session_config(file_name: &str, data: String) -> Result<()> {
    let path = get_config_file_path(file_name)?;
    fs::write(&path, data)?;
    Ok(())
}

pub fn load_session_from_config(file_name: &str) -> Result<String> {
    let path = get_config_file_path(file_name)?;
    let data = fs::read_to_string(path)?;
    Ok(data)
}

pub fn list_saved_sessions() -> Result<Vec<String>> {
    let dir_path = get_and_ensure_session_storage_dir()?;

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
    let mut path = get_and_ensure_session_storage_dir()?;
    path.push(format!("{file_name}.yaml"));
    Ok(path)
}

fn get_and_ensure_session_storage_dir() -> Result<PathBuf> {
    let dir_path = get_session_storage_dir_path()?;
    fs::create_dir_all(&dir_path).with_context(|| {
        format!("Failed to create directory {}", dir_path.display())
    })?;
    Ok(dir_path)
}

fn get_session_storage_dir_path() -> Result<PathBuf> {
    if let Ok(dir) = env::var("TSMAN_CONFIG_STORAGE_DIR") {
        return Ok(PathBuf::from(dir));
    }

    let home = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine HOME directory"))?;
    Ok(home.join(".config").join(DEFAULT_CONFIG_STORAGE_DIR))
}
