//! API for interacting with the disk
//!
//! This module contains functions for interacting with the tsman
//! storage directories (sessions and layouts).
use std::path::PathBuf;
use std::{env, fs};

use anyhow::{Context, Result};
use dirs::home_dir;

/// Default directory name inside `~/.config` for storing session configs.
const DEFAULT_SESSION_STORAGE_DIR: &str = ".tsessions";
/// Default directory name inside `~/.config` for storing layout configs.
const DEFAULT_LAYOUT_STORAGE_DIR: &str = ".tlayouts";

/// Determines which storage directory to use.
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

/// Saves a configuration to disk.
///
/// The configuration is written as a `.yaml` file in the storage
/// directory. The directory is automatically created if it doesn't exist.
///
/// # Arguments
/// * `kind` – Which storage directory to use.
/// * `file_name` – Base filename for the configuration file.
/// * `data` – YAML-formatted data.
///
/// # Errors
/// Returns an error if the storage directory cannot be determined or created,
/// or the file cannot be written.
pub fn save_config(
    kind: StorageKind,
    file_name: &str,
    data: String,
) -> Result<()> {
    let path = get_config_file_path(kind, file_name)?;
    fs::write(&path, data)?;
    Ok(())
}

/// Loads a configuration from disk.
///
/// Reads the `.yaml` file from the storage directory.
///
/// # Arguments
/// * `kind` – Which storage directory to use.
/// * `file_name` – Base filename for the configuration file.
///
/// # Returns
/// The file contents as a `String`.
///
/// # Errors
/// Returns an error if the storage directory cannot be determined,
/// or the file cannot be found or read.
pub fn load_config(kind: StorageKind, file_name: &str) -> Result<String> {
    let path = get_config_file_path(kind, file_name)?;
    let data = fs::read_to_string(path)?;
    Ok(data)
}

/// Lists all saved configurations in a storage directory.
///
/// This scans the storage directory and returns the base names of all files.
///
/// # Arguments
/// * `kind` – Which storage directory to use.
///
/// # Returns
/// A vector of config names.
///
/// # Errors
/// Returns an error if the storage directory cannot be determined or created,
/// the directory cannot be read, or any file name is invalid UTF-8.
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

/// Gets the full path to a configuration file.
///
/// The file is located in the storage directory and has a `.yaml` extension.
///
/// # Arguments
/// * `kind` – Which storage directory to use.
/// * `file_name` – Base filename.
///
/// # Returns
/// A [`PathBuf`] pointing to the configuration file.
///
/// # Errors
/// Returns an error if the storage directory cannot be determined or created.
pub fn get_config_file_path(
    kind: StorageKind,
    file_name: &str,
) -> Result<PathBuf> {
    let mut path = get_and_ensure_storage_dir(kind)?;
    path.push(format!("{file_name}.yaml"));
    Ok(path)
}

/// Gets the path of a storage dir, creating it if necessary.
///
/// # Errors
/// Returns an error if the directory cannot be determined or created.
fn get_and_ensure_storage_dir(kind: StorageKind) -> Result<PathBuf> {
    let dir_path = get_storage_dir_path(&kind)?;
    fs::create_dir_all(&dir_path).with_context(|| {
        format!("Failed to create directory {}", dir_path.display())
    })?;
    Ok(dir_path)
}

/// Determines the path of a storage directory.
///
/// Checks the appropriate environment variable first, then falls back to
/// `~/.config/<default_dir>`.
///
/// # Errors
/// Returns an error if the home directory cannot be determined.
fn get_storage_dir_path(kind: &StorageKind) -> Result<PathBuf> {
    if let Ok(dir) = env::var(kind.env_var()) {
        return Ok(PathBuf::from(dir));
    }

    let home = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine HOME directory"))?;
    Ok(home.join(".config").join(kind.default_dir()))
}
