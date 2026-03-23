use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_import_export::{export_gedcom, export_json};
use std::fs::File;
use std::io::BufWriter;

#[derive(Subcommand)]
pub enum ExportCommands {
    /// Export to GEDCOM 5.5 format
    Gedcom {
        /// Output file path
        output: String,
    },
    /// Export to JSON format
    Json {
        /// Output file path
        output: String,
    },
}

pub fn handle(cmd: ExportCommands, app: &Application) -> Result<()> {
    match cmd {
        ExportCommands::Gedcom { output } => {
            let file = File::create(&output)?;
            let mut writer = BufWriter::new(file);
            export_gedcom(&app.db, &mut writer)?;
            println!("Exported GEDCOM to {}", output);
        }
        ExportCommands::Json { output } => {
            let file = File::create(&output)?;
            let mut writer = BufWriter::new(file);
            export_json(&app.db, &mut writer)?;
            println!("Exported JSON to {}", output);
        }
    }
    Ok(())
}
