use kinforge_core::KinforgeError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub default_export_dir: Option<PathBuf>,
    pub backup_on_open: bool,
    pub max_backups: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("kinforge.db"),
            default_export_dir: None,
            backup_on_open: true,
            max_backups: 10,
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, KinforgeError> {
        let text = std::fs::read_to_string(path)?;
        toml::from_str(&text).map_err(|e| KinforgeError::Config(e.to_string()))
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), KinforgeError> {
        let text = toml::to_string_pretty(self)
            .map_err(|e| KinforgeError::Config(e.to_string()))?;
        std::fs::write(path, text)?;
        Ok(())
    }

    /// Load config from the default location (~/.config/kinforge/config.toml),
    /// or return defaults if no file exists.
    pub fn load_or_default() -> Self {
        if let Some(config_dir) = dirs_path() {
            let cfg_file = config_dir.join("config.toml");
            if cfg_file.exists() {
                return Self::load(&cfg_file).unwrap_or_default();
            }
        }
        Self::default()
    }
}

fn dirs_path() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("kinforge")
    })
}
