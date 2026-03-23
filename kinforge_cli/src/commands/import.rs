use anyhow::Result;
use clap::Subcommand;
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
            let stats = import_gedcom(&content, &app.db)?;
            println!(
                "Imported from {}: {} people, {} sources",
                input, stats.people, stats.sources
            );
        }
        ImportCommands::Json { input } => {
            let file = File::open(&input)?;
            let mut reader = BufReader::new(file);
            import_json(&app.db, &mut reader)?;
            println!("Imported JSON from {}", input);
        }
    }
    Ok(())
}
