//! Persistence layer for reading/writing session and layout YAML configs to disk.
use std::path::PathBuf;
use std::{env, fs};

use anyhow::{Context, Result};
use dirs::home_dir;

const DEFAULT_SESSION_STORAGE_DIR: &str = ".tsessions";
const DEFAULT_LAYOUT_STORAGE_DIR: &str = ".tlayouts";

/// Selects between session and layout storage directories.
#[derive(Clone, Copy)]
pub enum StorageKind {
    Session,
    Layout,
}

impl StorageKind {
    fn env_var(&self) -> &'static str {
        match self {
            StorageKind::Session => "TSMAN_CONFIG_STORAGE_DIR",
            StorageKind::Layout => "TSMAN_LAYOUT_STORAGE_DIR",
        }
    }

    fn default_dir(&self) -> &'static str {
        match self {
            StorageKind::Session => DEFAULT_SESSION_STORAGE_DIR,
            StorageKind::Layout => DEFAULT_LAYOUT_STORAGE_DIR,
        }
    }
}

/// Writes `data` as `<file_name>.yaml` in the storage directory, creating it if needed.
pub fn save_config(
    kind: StorageKind,
    file_name: &str,
    data: String,
) -> Result<()> {
    let path = get_config_file_path(kind, file_name)?;
    fs::write(&path, data)?;
    Ok(())
}

/// Reads `<file_name>.yaml` from the storage directory and returns its contents.
pub fn load_config(kind: StorageKind, file_name: &str) -> Result<String> {
    let path = get_config_file_path(kind, file_name)?;
    let data = fs::read_to_string(path)?;
    Ok(data)
}

/// Returns the base names (without `.yaml`) of all configs in the storage directory.
pub fn list_saved_configs(kind: StorageKind) -> Result<Vec<String>> {
    let dir_path = get_and_ensure_storage_dir(kind)?;

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

/// Returns the full path to `<file_name>.yaml` in the storage directory.
pub fn get_config_file_path(
    kind: StorageKind,
    file_name: &str,
) -> Result<PathBuf> {
    let mut path = get_and_ensure_storage_dir(kind)?;
    path.push(format!("{file_name}.yaml"));
    Ok(path)
}

fn get_and_ensure_storage_dir(kind: StorageKind) -> Result<PathBuf> {
    let dir_path = get_storage_dir_path(&kind)?;
    fs::create_dir_all(&dir_path).with_context(|| {
        format!("Failed to create directory {}", dir_path.display())
    })?;
    Ok(dir_path)
}

fn get_storage_dir_path(kind: &StorageKind) -> Result<PathBuf> {
    if let Ok(dir) = env::var(kind.env_var()) {
        return Ok(PathBuf::from(dir));
    }

    let home = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine HOME directory"))?;
    Ok(home.join(".config").join(kind.default_dir()))
}
