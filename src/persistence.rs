use std::path::PathBuf;
use std::{env, fs};

use anyhow::{Context, Result};
use dirs::home_dir;

/// Default directory name inside `~/.config` for storing session configs.
const DEFAULT_CONFIG_STORAGE_DIR: &str = ".tsessions";

/// Saves a session configuration to disk.
///
/// The configuration is written as a `.yaml` file in the session storage
/// directory. The directory is automatically created if it doesn't exist.
///
/// # Arguments
/// * `file_name` – Base filename for the configuration file.
/// * `data` – YAML-formatted session data.
///
/// # Returns
/// `Ok(())` on success.
///
/// # Errors
/// Returns an error if:
/// - The storage directory cannot be determined or created.
/// - The file cannot be written.
pub fn save_session_config(file_name: &str, data: String) -> Result<()> {
    let path = get_config_file_path(file_name)?;
    fs::write(&path, data)?;
    Ok(())
}

/// Loads a session configuration from disk.
///
/// Reads the `.yaml` file from the session storage directory.
///
/// # Arguments
/// * `file_name` – Base filename for the configuration file.
///
/// # Returns
/// The file contents as a `String`.
///
/// # Errors
/// Returns an error if:
/// - The storage directory cannot be determined.
/// - The file cannot be found or read.
pub fn load_session_from_config(file_name: &str) -> Result<String> {
    let path = get_config_file_path(file_name)?;
    let data = fs::read_to_string(path)?;
    Ok(data)
}

/// Lists all saved session configurations.
///
/// This scans the storage directory and returns the base names of all files.
///
/// # Returns
/// A vector of session names.
///
/// # Errors
/// Returns an error if:
/// - The storage directory cannot be determined or created.
/// - The directory cannot be read.
/// - Any file name is invalid UTF-8.
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

/// Gets the full path to a session configuration file.
///
/// The file is located in the storage directory and has a `.yaml` extension.
///
/// # Arguments
/// * `file_name` – Base filename.
///
/// # Returns
/// A [`PathBuf`] pointing to the configuration file.
///
/// # Errors
/// Returns an error if the storage directory cannot be determined or created.
pub fn get_config_file_path(file_name: &str) -> Result<PathBuf> {
    let mut path = get_and_ensure_session_storage_dir()?;
    path.push(format!("{file_name}.yaml"));
    Ok(path)
}

/// Gets the path of the session storage dir, creating it if neccessary.
///
/// # Returns
/// The path to the storage directory.
///
/// # Errors
/// Returns an error if the directory cannot be determined or created.
fn get_and_ensure_session_storage_dir() -> Result<PathBuf> {
    let dir_path = get_session_storage_dir_path()?;
    fs::create_dir_all(&dir_path).with_context(|| {
        format!("Failed to create directory {}", dir_path.display())
    })?;
    Ok(dir_path)
}

/// Determines the path of the session storage directory.
///
/// If the `TSMAN_CONFIG_STORAGE_DIR` environment variable is set, that path
/// is used. Otherwise, the default path is:
/// `~/.config/DEFAULT_CONFIG_STORAGE_DIR`
///
/// # Returns
/// A [`PathBuf`] with the directory path.
///
/// # Errors
/// Returns an error if the home directory cannot be determined.
fn get_session_storage_dir_path() -> Result<PathBuf> {
    if let Ok(dir) = env::var("TSMAN_CONFIG_STORAGE_DIR") {
        return Ok(PathBuf::from(dir));
    }

    let home = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine HOME directory"))?;
    Ok(home.join(".config").join(DEFAULT_CONFIG_STORAGE_DIR))
}
