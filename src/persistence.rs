//! Persistence layer for reading/writing session and layout YAML configs to disk.
use std::path::PathBuf;
use std::{env, fs};

use anyhow::{Context, Result};
use dirs::home_dir;

use crate::config::StorageConfig;

const DEFAULT_SESSION_STORAGE_DIR: &str = ".tsessions";
const DEFAULT_LAYOUT_STORAGE_DIR: &str = ".tlayouts";

const ENV_SESSION_DIR: &str = "TSMAN_CONFIG_STORAGE_DIR";
const ENV_LAYOUT_DIR: &str = "TSMAN_LAYOUT_STORAGE_DIR";

/// Selects between session and layout storage directories.
#[derive(Clone, Copy)]
pub enum StorageKind {
    Session,
    Layout,
}

/// Persistence context - resolved storage directories.
pub struct Persistence {
    sessions_dir: PathBuf,
    layouts_dir: PathBuf,
}

impl Persistence {
    pub fn new(storage: &StorageConfig) -> Result<Self> {
        Ok(Self {
            sessions_dir: resolve_dir(
                ENV_SESSION_DIR,
                storage.sessions_dir.as_deref(),
                DEFAULT_SESSION_STORAGE_DIR,
            )?,
            layouts_dir: resolve_dir(
                ENV_LAYOUT_DIR,
                storage.layouts_dir.as_deref(),
                DEFAULT_LAYOUT_STORAGE_DIR,
            )?,
        })
    }

    fn dir(&self, kind: StorageKind) -> &PathBuf {
        match kind {
            StorageKind::Session => &self.sessions_dir,
            StorageKind::Layout => &self.layouts_dir,
        }
    }

    /// Writes `data` as `<file_name>.yaml` in the storage directory.
    pub fn save_config(
        &self,
        kind: StorageKind,
        file_name: &str,
        data: String,
    ) -> Result<()> {
        let path = self.get_config_file_path(kind, file_name)?;
        fs::write(&path, data)?;
        Ok(())
    }

    /// Reads `<file_name>.yaml` from the storage directory.
    pub fn load_config(
        &self,
        kind: StorageKind,
        file_name: &str,
    ) -> Result<String> {
        let path = self.get_config_file_path(kind, file_name)?;
        let data = fs::read_to_string(path)?;
        Ok(data)
    }

    /// Returns the base names (without `.yaml`) of all configs in the
    /// storage directory.
    pub fn list_saved_configs(&self, kind: StorageKind) -> Result<Vec<String>> {
        let dir_path = self.ensure_dir(kind)?;

        let paths = fs::read_dir(dir_path.into_os_string())?;
        let mut result = Vec::with_capacity(paths.size_hint().0);

        for entry in paths {
            let path = entry?.path();

            let name = path
                .file_stem()
                .ok_or_else(|| {
                    anyhow::anyhow!("Missing file stem for {:?}", path)
                })?
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
        &self,
        kind: StorageKind,
        file_name: &str,
    ) -> Result<PathBuf> {
        let mut path = self.ensure_dir(kind)?;
        path.push(format!("{file_name}.yaml"));
        Ok(path)
    }

    fn ensure_dir(&self, kind: StorageKind) -> Result<PathBuf> {
        let dir = self.dir(kind);
        fs::create_dir_all(dir).with_context(|| {
            format!("Failed to create directory {}", dir.display())
        })?;
        Ok(dir.clone())
    }
}

fn resolve_dir(
    env_var: &str,
    config_override: Option<&std::path::Path>,
    default_name: &str,
) -> Result<PathBuf> {
    if let Ok(val) = env::var(env_var) {
        return Ok(PathBuf::from(val));
    }
    if let Some(path) = config_override {
        return Ok(path.to_path_buf());
    }
    let home = home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine HOME directory"))?;
    Ok(home.join(".config").join(default_name))
}
