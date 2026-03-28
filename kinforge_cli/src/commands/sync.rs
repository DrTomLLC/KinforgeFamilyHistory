use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_sync::{FileSyncBackend, SyncBackend};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum SyncCommands {
    /// Push the local database to a sync directory (file/network share)
    Push {
        /// Path to the shared sync directory
        #[arg(value_name = "DIR")]
        dir: PathBuf,

        /// Stable identifier for this device (defaults to hostname or random UUID)
        #[arg(long, default_value = "local")]
        device_id: String,
    },

    /// Pull records from a sync directory into the local database (additive only)
    Pull {
        /// Path to the shared sync directory
        #[arg(value_name = "DIR")]
        dir: PathBuf,
    },

    /// Compare local database counts against the remote sync manifest
    Status {
        /// Path to the shared sync directory
        #[arg(value_name = "DIR")]
        dir: PathBuf,
    },
}

pub fn handle(cmd: SyncCommands, app: &Application) -> Result<()> {
    match cmd {
        SyncCommands::Push { dir, device_id } => {
            println!(
                "\n{}\n{}",
                "  Sync — Push  ".bold().bright_cyan().on_black(),
                "─".repeat(48).bright_black()
            );
            println!("  Sync dir : {}", dir.display().to_string().bright_white());
            println!("  Device   : {}", device_id.bright_white());
            println!();

            let backend = FileSyncBackend::with_device_id(&dir, &device_id);
            let result = backend.push(app.database())?;

            println!(
                "  {} {}",
                "✔".bright_green().bold(),
                format!("Pushed {} records to {}", result.records_pushed, dir.display())
                    .bright_white()
            );
            println!(
                "\n  {}",
                "Files written:".bright_black()
            );
            println!(
                "    {}",
                dir.join(kinforge_sync::EXPORT_FILE)
                    .display()
                    .to_string()
                    .bright_black()
            );
            println!(
                "    {}",
                dir.join(kinforge_sync::MANIFEST_FILE)
                    .display()
                    .to_string()
                    .bright_black()
            );
            println!();
        }

        SyncCommands::Pull { dir } => {
            println!(
                "\n{}\n{}",
                "  Sync — Pull  ".bold().bright_cyan().on_black(),
                "─".repeat(48).bright_black()
            );
            println!("  Sync dir : {}", dir.display().to_string().bright_white());
            println!();

            let backend = FileSyncBackend::new(&dir);
            let result = backend.pull(app.database())?;

            println!(
                "  {} {}",
                "✔".bright_green().bold(),
                format!(
                    "Pulled {} records from {}",
                    result.records_pulled,
                    dir.display()
                )
                .bright_white()
            );
            println!(
                "\n  {}",
                "Note: existing records were not overwritten (additive merge)."
                    .bright_black()
            );
            println!();
        }

        SyncCommands::Status { dir } => {
            println!(
                "\n{}\n{}",
                "  Sync — Status  ".bold().bright_cyan().on_black(),
                "─".repeat(48).bright_black()
            );
            println!("  Sync dir : {}", dir.display().to_string().bright_white());
            println!();

            let backend = FileSyncBackend::new(&dir);
            let status = backend.status(app.database())?;

            println!("  {}", "Local database:".bold());
            println!("    People        : {}", status.local_people.to_string().bright_white());
            println!("    Events        : {}", status.local_events.to_string().bright_white());
            println!("    Sources       : {}", status.local_sources.to_string().bright_white());
            println!(
                "    Relationships : {}",
                status.local_relationships.to_string().bright_white()
            );
            println!();

            if status.remote_device_id.is_none() {
                println!(
                    "  {}",
                    "Remote: no manifest found — has this directory been pushed to yet?"
                        .yellow()
                );
            } else {
                println!("  {}", "Remote snapshot:".bold());
                println!(
                    "    People        : {}",
                    status.remote_people.to_string().bright_white()
                );
                println!(
                    "    Events        : {}",
                    status.remote_events.to_string().bright_white()
                );
                println!(
                    "    Sources       : {}",
                    status.remote_sources.to_string().bright_white()
                );
                println!(
                    "    Relationships : {}",
                    status.remote_relationships.to_string().bright_white()
                );
                if let Some(pushed_at) = status.remote_pushed_at {
                    println!(
                        "    Pushed at     : {}",
                        pushed_at
                            .format("%Y-%m-%d %H:%M UTC")
                            .to_string()
                            .bright_black()
                    );
                }
                if let Some(device_id) = &status.remote_device_id {
                    println!(
                        "    Device ID     : {}",
                        device_id.bright_black()
                    );
                }

                // Diff hints
                let people_diff =
                    status.remote_people as i64 - status.local_people as i64;
                let events_diff =
                    status.remote_events as i64 - status.local_events as i64;

                if people_diff != 0 || events_diff != 0 {
                    println!();
                    if people_diff > 0 || events_diff > 0 {
                        println!(
                            "  {} remote has more records — run {} to import them",
                            "→".bright_yellow(),
                            "kinforge sync pull <dir>".bright_white()
                        );
                    } else {
                        println!(
                            "  {} local has more records — run {} to export them",
                            "→".bright_yellow(),
                            "kinforge sync push <dir>".bright_white()
                        );
                    }
                } else {
                    println!("\n  {} Databases appear to be in sync.", "✔".bright_green());
                }
            }
            println!();
        }
    }
    Ok(())
}
