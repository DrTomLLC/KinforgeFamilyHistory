use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_import_export::{import_gedcom, import_json, DuplicateHandling, ImportOptions};
use std::fs::File;
use std::io::BufReader;

#[derive(Subcommand)]
pub enum ImportCommands {
    /// Import from a GEDCOM 5.5 file
    Gedcom {
        /// Input file path
        input: String,
        /// How to handle people already in the database: skip (default), merge, add
        #[arg(long, default_value = "skip")]
        on_duplicate: String,
    },
    /// Import from a Kinforge JSON file
    Json {
        /// Input file path
        input: String,
    },
}

pub fn handle(cmd: ImportCommands, app: &Application) -> Result<()> {
    match cmd {
        ImportCommands::Gedcom {
            input,
            on_duplicate,
        } => {
            let dup = match on_duplicate.to_lowercase().as_str() {
                "skip" | "s" => DuplicateHandling::Skip,
                "merge" | "m" => DuplicateHandling::Merge,
                "add" | "a" => DuplicateHandling::Add,
                other => anyhow::bail!(
                    "Unknown --on-duplicate value '{}'. Use: skip, merge, add",
                    other
                ),
            };
            let opts = ImportOptions { on_duplicate: dup };
            let content = std::fs::read_to_string(&input)?;
            let stats = import_gedcom(&content, &app.db, &opts)?;
            println!(
                "Imported from {}: {} people, {} events, {} sources, {} relationships",
                input, stats.people, stats.events, stats.sources, stats.relationships
            );
            if stats.skipped_duplicates > 0 {
                println!("  Skipped {} duplicate(s).", stats.skipped_duplicates);
            }
            if stats.merged_people > 0 {
                println!("  Merged {} duplicate(s).", stats.merged_people);
            }
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
