use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;

#[derive(Subcommand)]
pub enum BackupCommands {
    /// List all existing backup files (newest first)
    List,
    /// Create a manual backup of the database right now
    Create,
}

pub fn handle(cmd: BackupCommands, app: &Application) -> Result<()> {
    match cmd {
        BackupCommands::List => {
            let backups = app.list_backups()?;
            if backups.is_empty() {
                println!("{}", "  No backups found.  ".bold().bright_black().on_black());
                println!(
                    "{}",
                    "  Run 'kinforge backup create' or set backup_on_open = true in config."
                        .bright_black()
                );
                return Ok(());
            }

            println!(
                "\n{}",
                format!("  {} backup(s)  ", backups.len())
                    .bold()
                    .bright_cyan()
                    .on_black()
            );
            println!();

            for (i, b) in backups.iter().enumerate() {
                let size_str = format_size(b.size_bytes);
                let name = b
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?");
                let prefix = if i == 0 { "latest" } else { "      " };
                println!(
                    "  {} {}  {}  {}",
                    if i == 0 {
                        prefix.bright_green().bold().to_string()
                    } else {
                        prefix.bright_black().to_string()
                    },
                    name.bold(),
                    size_str.yellow(),
                    b.path.display().to_string().bright_black()
                );
            }
            println!();
        }

        BackupCommands::Create => {
            let path = app.backup_now()?;
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("backup");
            println!(
                "{} {}",
                "\u{2713} Backup created:".bright_green().bold(),
                name.bold()
            );
            println!("  {}", path.display().to_string().bright_black());
        }
    }
    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{} B", bytes)
    }
}
