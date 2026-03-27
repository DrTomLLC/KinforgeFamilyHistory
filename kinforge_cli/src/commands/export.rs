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
    /// Export people list to CSV (id, given, surname, sex, birth_year, death_year)
    Csv {
        /// Output file path
        output: String,
    },
    /// Export all events to CSV (person_id, person_name, event_type, date, place)
    EventsCsv {
        /// Output file path
        output: String,
    },
    /// Export all sources to CSV (id, title, author, year, citation_count)
    SourcesCsv {
        /// Output file path
        output: String,
    },
    /// Export all people as a self-contained single-file HTML document
    Html {
        /// Output file path (e.g. family.html)
        output: String,
    },
    /// Export places with coordinates as GeoJSON
    Geojson {
        /// Output file path (e.g. places.geojson)
        output: String,
    },
}

pub fn handle(cmd: ExportCommands, app: &Application) -> Result<()> {
    match cmd {
        ExportCommands::Gedcom { output } => {
            let file = File::create(&output)?;
            let mut writer = BufWriter::new(file);
            export_gedcom(app.database(), &mut writer)?;
            println!(
                "{} {}",
                "Exported GEDCOM \u{2192}".green().bold(),
                output.bold()
            );
        }
        ExportCommands::Json { output } => {
            let file = File::create(&output)?;
            let mut writer = BufWriter::new(file);
            export_json(app.database(), &mut writer)?;
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

        ExportCommands::EventsCsv { output } => {
            let people = app.list_people()?;
            let file = File::create(&output)?;
            let mut writer = BufWriter::new(file);
            writeln!(writer, "person_id,person_name,event_type,date,place")?;

            let mut row_count = 0usize;
            let escape = |s: &str| -> String {
                if s.contains(',') || s.contains('"') || s.contains('\n') {
                    format!("\"{}\"", s.replace('"', "\"\""))
                } else {
                    s.to_string()
                }
            };
            let id_str = |id: &kinforge_core::models::PersonId| id.as_str();

            for p in &people {
                let name = p.display_name();
                let events = app.list_events_for_person(&p.id).unwrap_or_default();
                for ev in &events {
                    let date_str = ev
                        .date
                        .as_ref()
                        .map(|d| d.to_string())
                        .unwrap_or_default();
                    let place_str = ev
                        .place_id
                        .as_ref()
                        .and_then(|pid| app.get_place(pid).ok())
                        .map(|pl| pl.name)
                        .unwrap_or_default();
                    writeln!(
                        writer,
                        "{},{},{},{},{}",
                        escape(&id_str(&p.id)),
                        escape(&name),
                        escape(&ev.event_type.to_string()),
                        escape(&date_str),
                        escape(&place_str)
                    )?;
                    row_count += 1;
                }
            }
            writer.flush()?;
            println!(
                "{} {} {}",
                "Exported events CSV \u{2192}".green().bold(),
                output.bold(),
                format!("({} events)", row_count).bright_black()
            );
        }

        ExportCommands::SourcesCsv { output } => {
            let sources = app.list_sources()?;
            let file = File::create(&output)?;
            let mut writer = BufWriter::new(file);
            writeln!(writer, "id,title,author,year,citation_count")?;
            let escape = |s: &str| -> String {
                if s.contains(',') || s.contains('"') || s.contains('\n') {
                    format!("\"{}\"", s.replace('"', "\"\""))
                } else {
                    s.to_string()
                }
            };
            for s in &sources {
                let citation_count = app.list_citations_for_source(&s.id)
                    .map(|v| v.len()).unwrap_or(0);
                writeln!(
                    writer,
                    "{},{},{},{},{}",
                    escape(&s.id.as_str()),
                    escape(&s.title),
                    escape(s.author.as_deref().unwrap_or("")),
                    s.year.map(|y| y.to_string()).unwrap_or_default(),
                    citation_count
                )?;
            }
            writer.flush()?;
            println!(
                "{} {} {}",
                "Exported sources CSV \u{2192}".green().bold(),
                output.bold(),
                format!("({} sources)", sources.len()).bright_black()
            );
        }

        ExportCommands::Html { output } => {
            let html = html_export(app.database())?;
            std::fs::write(&output, &html)?;
            let people = app.list_people()?;
            println!(
                "{} {} {}",
                "Exported HTML \u{2192}".green().bold(),
                output.bold(),
                format!("({} people, {} bytes)", people.len(), html.len()).bright_black()
            );
        }

        ExportCommands::Geojson { output } => {
            let places = app.list_places()?;
            let features: Vec<String> = places
                .iter()
                .filter_map(|p| {
                    let lat = p.latitude?;
                    let lon = p.longitude?;
                    Some(format!(
                        "    {{\"type\":\"Feature\",\"geometry\":{{\"type\":\"Point\",\"coordinates\":[{lon},{lat}]}},\"properties\":{{\"id\":\"{id}\",\"name\":\"{name}\"}}}}",
                        lon = lon,
                        lat = lat,
                        id = p.id,
                        name = p.name.replace('"', "\\\""),
                    ))
                })
                .collect();
            let geojson = format!(
                "{{\n  \"type\": \"FeatureCollection\",\n  \"features\": [\n{}\n  ]\n}}\n",
                features.join(",\n")
            );
            std::fs::write(&output, &geojson)?;
            println!(
                "{} {} {}",
                "Exported GeoJSON \u{2192}".green().bold(),
                output.bold(),
                format!("({} places with coordinates)", features.len()).bright_black()
            );
        }
    }
    Ok(())
}
