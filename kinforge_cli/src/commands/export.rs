use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_core::models::{EventDate, EventType};
use kinforge_import_export::{export_gedcom, export_json};
use kinforge_reports::html_export;
use std::fs::File;
use std::io::{BufWriter, Write};

#[derive(Subcommand)]
pub enum ExportCommands {
    /// Export to GEDCOM 5.5 format
    Gedcom {
        /// Output file path
        output: String,
    },
    /// Export to Kinforge JSON format
    Json {
        /// Output file path
        output: String,
    },
    /// Export people list to CSV (id, name, sex, birth_year, death_year)
    Csv {
        /// Output file path
        output: String,
    },
    /// Export all people as a self-contained single-file HTML document
    Html {
        /// Output file path (e.g. family.html)
        output: String,
    },
}

pub fn handle(cmd: ExportCommands, app: &Application) -> Result<()> {
    match cmd {
        ExportCommands::Gedcom { output } => {
            let file = File::create(&output)?;
            let mut writer = BufWriter::new(file);
            export_gedcom(&app.db, &mut writer)?;
            println!(
                "{} {}",
                "Exported GEDCOM \u{2192}".green().bold(),
                output.bold()
            );
        }
        ExportCommands::Json { output } => {
            let file = File::create(&output)?;
            let mut writer = BufWriter::new(file);
            export_json(&app.db, &mut writer)?;
            println!(
                "{} {}",
                "Exported JSON \u{2192}".green().bold(),
                output.bold()
            );
        }
        ExportCommands::Csv { output } => {
            let people = app.list_people()?;
            let file = File::create(&output)?;
            let mut writer = BufWriter::new(file);

            writeln!(writer, "id,given,surname,sex,birth_year,death_year")?;

            for p in &people {
                let given = p
                    .names
                    .first()
                    .and_then(|n| n.given.as_deref())
                    .unwrap_or("");
                let surname = p
                    .names
                    .first()
                    .and_then(|n| n.surname.as_deref())
                    .unwrap_or("");
                let events = app.list_events_for_person(&p.id).unwrap_or_default();

                let birth_year = events
                    .iter()
                    .find(|e| matches!(e.event_type, EventType::Birth))
                    .and_then(|e| e.date.as_ref())
                    .and_then(|d| match d {
                        EventDate::Exact(nd) | EventDate::Approximate(nd) => {
                            Some(nd.format("%Y").to_string())
                        }
                        _ => None,
                    })
                    .unwrap_or_default();

                let death_year = events
                    .iter()
                    .find(|e| matches!(e.event_type, EventType::Death))
                    .and_then(|e| e.date.as_ref())
                    .and_then(|d| match d {
                        EventDate::Exact(nd) | EventDate::Approximate(nd) => {
                            Some(nd.format("%Y").to_string())
                        }
                        _ => None,
                    })
                    .unwrap_or_default();

                // Escape commas in name fields
                let escape = |s: &str| {
                    if s.contains(',') || s.contains('"') {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s.to_string()
                    }
                };

                writeln!(
                    writer,
                    "{},{},{},{},{},{}",
                    p.id,
                    escape(given),
                    escape(surname),
                    p.sex,
                    birth_year,
                    death_year
                )?;
            }

            println!(
                "{} {} {}",
                "Exported CSV \u{2192}".green().bold(),
                output.bold(),
                format!("({} people)", people.len()).bright_black()
            );
        }

        ExportCommands::Html { output } => {
            let html = html_export(&app.db)?;
            std::fs::write(&output, &html)?;
            let people = app.list_people()?;
            println!(
                "{} {} {}",
                "Exported HTML \u{2192}".green().bold(),
                output.bold(),
                format!("({} people, {} bytes)", people.len(), html.len()).bright_black()
            );
        }
    }
    Ok(())
}
