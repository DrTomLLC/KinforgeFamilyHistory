use anyhow::{bail, Result};
use clap::Subcommand;
use colored::Colorize;
use kinforge_config::Config;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Print the active configuration and data paths
    Show,
    /// Write a default config file (creates it if it does not exist)
    Init,
    /// Set a configuration value and save to disk
    ///
    /// Keys: database-path, backup-on-open, max-backups, log-level, export-dir
    Set {
        key: String,
        value: String,
    },
}

pub fn handle(cmd: ConfigCommands, config: &Config) -> Result<()> {
    match cmd {
        ConfigCommands::Show => {
            print_config(config);
        }

        ConfigCommands::Init => {
            let path = Config::default_config_path()
                .ok_or_else(|| anyhow::anyhow!("Cannot determine config file path"))?;
            if path.exists() {
                println!(
                    "{} {}",
                    "Config already exists at".yellow(),
                    path.display().to_string().bold()
                );
            } else {
                let default = Config::default();
                default.save(&path).map_err(|e| anyhow::anyhow!("{}", e))?;
                println!(
                    "{} {}",
                    "Created default config at".green().bold(),
                    path.display().to_string().bold()
                );
            }
            print_config(config);
        }

        ConfigCommands::Set { key, value } => {
            let path = Config::default_config_path()
                .ok_or_else(|| anyhow::anyhow!("Cannot determine config file path"))?;

            // Load existing config or start from default
            let mut cfg = if path.exists() {
                Config::load(&path).map_err(|e| anyhow::anyhow!("{}", e))?
            } else {
                Config::default()
            };

            match key.to_lowercase().replace('_', "-").as_str() {
                "database-path" => {
                    cfg.database_path = PathBuf::from(&value);
                }
                "backup-on-open" => {
                    cfg.backup_on_open = value
                        .parse::<bool>()
                        .map_err(|_| anyhow::anyhow!("Expected true or false"))?;
                }
                "max-backups" => {
                    cfg.max_backups = value
                        .parse::<u32>()
                        .map_err(|_| anyhow::anyhow!("Expected a positive integer"))?;
                }
                "log-level" => {
                    let lvl = value.to_lowercase();
                    if !["error", "warn", "info", "debug", "trace"].contains(&lvl.as_str()) {
                        bail!("log-level must be one of: error, warn, info, debug, trace");
                    }
                    cfg.log_level = lvl;
                }
                "export-dir" => {
                    cfg.default_export_dir = Some(PathBuf::from(&value));
                }
                other => {
                    bail!(
                        "Unknown config key '{}'. Valid keys: database-path, backup-on-open, max-backups, log-level, export-dir",
                        other
                    );
                }
            }

            cfg.save(&path).map_err(|e| anyhow::anyhow!("{}", e))?;
            println!(
                "{} {} = {}",
                "Set".green().bold(),
                key.cyan(),
                value.yellow()
            );
            println!(
                "  {} {}",
                "Saved to".bright_black(),
                path.display().to_string().bright_black()
            );
        }
    }
    Ok(())
}

fn print_config(config: &Config) {
    println!(
        "{}\n",
        "  Kinforge Configuration  ".bold().bright_cyan().on_black()
    );
    println!(
        "  {} {}",
        "Database path:  ".cyan(),
        config.database_path.display().to_string().bold()
    );
    println!(
        "  {} {}",
        "Backup on open: ".cyan(),
        config.backup_on_open.to_string().yellow()
    );
    println!(
        "  {} {}",
        "Max backups:    ".cyan(),
        config.max_backups.to_string().yellow()
    );
    println!(
        "  {} {}",
        "Log level:      ".cyan(),
        config.log_level.bright_black()
    );
    if let Some(ref dir) = config.default_export_dir {
        println!(
            "  {} {}",
            "Export dir:     ".cyan(),
            dir.display().to_string().bold()
        );
    }
    if let Some(p) = Config::default_config_path() {
        println!(
            "  {} {}",
            "Config file:    ".cyan(),
            p.display().to_string().bright_black()
        );
    }
}
