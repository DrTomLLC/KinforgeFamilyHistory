use kinforge_core::KinforgeError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the SQLite database file.
    pub database_path: PathBuf,
    /// Directory to write exported files. Defaults to current directory.
    pub default_export_dir: Option<PathBuf>,
    /// Create a timestamped backup on every open.
    pub backup_on_open: bool,
    /// Maximum number of backup files to keep.
    pub max_backups: u32,
    /// Log level: "error", "warn", "info", "debug", "trace"
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_path: default_database_path(),
            default_export_dir: None,
            backup_on_open: true,
            max_backups: 10,
            log_level: "warn".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, KinforgeError> {
        let text = std::fs::read_to_string(path)?;
        toml::from_str(&text).map_err(|e| KinforgeError::Config(e.to_string()))
    }

    /// Save configuration to a TOML file, creating parent directories as needed.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), KinforgeError> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text =
            toml::to_string_pretty(self).map_err(|e| KinforgeError::Config(e.to_string()))?;
        std::fs::write(path, text)?;
        Ok(())
    }

    /// Load from the user config file, or return defaults if no file exists.
    pub fn load_or_default() -> Self {
        if let Some(path) = user_config_file() {
            if path.exists() {
                match Self::load(&path) {
                    Ok(cfg) => return cfg,
                    Err(e) => {
                        eprintln!("Warning: failed to read config {:?}: {}", path, e);
                    }
                }
            }
        }
        Self::default()
    }

    /// Return the path of the default config file location.
    pub fn default_config_path() -> Option<PathBuf> {
        user_config_file()
    }

    /// Return the data directory where the default database lives.
    pub fn data_dir() -> PathBuf {
        kinforge_data_dir()
    }
}

/// `~/.local/share/kinforge/kinforge.db` (Linux/macOS)
/// or `%APPDATA%\kinforge\kinforge.db` (Windows)
fn default_database_path() -> PathBuf {
    kinforge_data_dir().join("kinforge.db")
}

fn kinforge_data_dir() -> PathBuf {
    // Prefer XDG_DATA_HOME, then fall back to ~/.local/share
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("kinforge");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("kinforge");
    }
    // Last resort: current directory
    PathBuf::from(".")
}

/// `~/.config/kinforge/config.toml`
fn user_config_file() -> Option<PathBuf> {
    // Prefer XDG_CONFIG_HOME
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("kinforge").join("config.toml"));
    }
    std::env::var("HOME").ok().map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("kinforge")
            .join("config.toml")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert!(cfg.backup_on_open);
        assert_eq!(cfg.max_backups, 10);
        assert_eq!(cfg.log_level, "warn");
        assert!(cfg.database_path.ends_with("kinforge.db"));
    }

    #[test]
    fn test_roundtrip() {
        let cfg = Config::default();
        let dir = std::env::temp_dir().join("kinforge_config_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        cfg.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.database_path, cfg.database_path);
        assert_eq!(loaded.max_backups, cfg.max_backups);
        std::fs::remove_file(path).ok();
        std::fs::remove_dir(dir).ok();
    }
}
