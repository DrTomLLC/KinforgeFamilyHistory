use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_import_export::{import_gedcom, import_json};
use std::fs::File;
use std::io::BufReader;

#[derive(Subcommand)]
pub enum ImportCommands {
    /// Import from a GEDCOM 5.5 file
    Gedcom {
        /// Input file path
        input: String,
    },
    /// Import from a Kinforge JSON file
    Json {
        /// Input file path
        input: String,
    },
}

pub fn handle(cmd: ImportCommands, app: &Application) -> Result<()> {
    match cmd {
        ImportCommands::Gedcom { input } => {
            let content = std::fs::read_to_string(&input)?;
            let stats = import_gedcom(&content, app.database())?;
            println!(
                "{} {} {} {}, {}, {}",
                "Imported GEDCOM from".green().bold(),
                input.bold(),
                "\u{2014}".bright_black(),
                format!("{} people", stats.people).bold(),
                format!("{} sources", stats.sources).bold(),
                "added".bright_black()
            );
            if stats.duplicates_skipped > 0 {
                println!(
                    "  {} {}",
                    format!("{} duplicate(s) skipped", stats.duplicates_skipped).yellow(),
                    "(same name + birth year already exists)".bright_black()
                );
            }
        }
        ImportCommands::Json { input } => {
            let file = File::open(&input)?;
            let mut reader = BufReader::new(file);
            let stats = import_json(app.database(), &mut reader)?;
            println!(
                "{} {} {} {}, {}, {}, {}",
                "Imported JSON from".green().bold(),
                input.bold(),
                "\u{2014}".bright_black(),
                format!("{} people", stats.people).bold(),
                format!("{} events", stats.events).bold(),
                format!("{} sources", stats.sources).bold(),
                "added".bright_black()
            );
            if stats.places > 0 || stats.relationships > 0 {
                println!(
                    "  {}",
                    format!(
                        "{} place(s), {} relationship(s) imported",
                        stats.places, stats.relationships
                    )
                    .bright_black()
                );
            }
        }
    }
    Ok(())
}
