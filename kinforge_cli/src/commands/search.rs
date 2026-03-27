use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_query::{EventQuery, PersonQuery, SourceQuery};

#[derive(Subcommand)]
pub enum SearchCommands {
    /// Search people by name (and optionally sex)
    People {
        query: String,
        #[arg(long)]
        sex: Option<String>,
    },
    /// Search sources by title or author
    Sources {
        query: String,
        #[arg(long)]
        from_year: Option<i32>,
        #[arg(long)]
        to_year: Option<i32>,
    },
    /// Search person notes and event notes for a keyword
    Notes { query: String },
    /// Search events by place name (optional: filter by event type)
    Events {
        /// Place name fragment to search
        #[arg(long)]
        place: Option<String>,
        /// Event type filter (birth, death, marriage, etc.)
        #[arg(long)]
        event_type: Option<String>,
    },
}

pub fn handle(cmd: SearchCommands, app: &Application) -> Result<()> {
    match cmd {
        SearchCommands::People { query, sex } => {
            let mut q = PersonQuery::new().name_contains(&query);
            if let Some(s) = sex {
                q = q.sex(s.parse()?);
            }
            let results = q.run(&app.db)?;
            if results.is_empty() {
                println!(
                    "{} \u{2018}{}\u{2019}",
                    "No people matching".bright_black(),
                    query.yellow()
                );
            } else {
                println!(
                    "{}\n",
                    format!("  {} result(s)  ", results.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for p in &results {
                    println!(
                        "  {} {} {}",
                        p.id.to_string().bright_black(),
                        p.display_name().bold(),
                        format!("({})", p.sex).bright_black()
                    );
                }
            }
        }

        SearchCommands::Sources {
            query,
            from_year,
            to_year,
        } => {
            let mut q = SourceQuery::new().title_contains(&query);
            if let (Some(f), Some(t)) = (from_year, to_year) {
                q = q.year_range(f, t);
            }
            let results = q.run(&app.db)?;
            if results.is_empty() {
                println!(
                    "{} \u{2018}{}\u{2019}",
                    "No sources matching".bright_black(),
                    query.yellow()
                );
            } else {
                println!(
                    "{}\n",
                    format!("  {} result(s)  ", results.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for s in &results {
                    let year = s
                        .year
                        .map(|y| format!(" {}", format!("({})", y).yellow()))
                        .unwrap_or_default();
                    println!(
                        "  {} {}{}",
                        s.id.to_string().bright_black(),
                        s.title.bold(),
                        year
                    );
                }
            }
        }

        SearchCommands::Notes { query } => {
            let results = app.search_notes(&query)?;
            if results.is_empty() {
                println!(
                    "{} \u{2018}{}\u{2019}",
                    "No notes matching".bright_black(),
                    query.yellow()
                );
            } else {
                println!(
                    "{}\n",
                    format!("  {} match(es)  ", results.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for m in &results {
                    println!(
                        "  {} {} {} {}",
                        m.kind.bright_cyan(),
                        m.id.bright_black(),
                        "\u{2014}".bright_black(),
                        m.label.bold()
                    );
                    // Show a one-line excerpt with the match highlighted
                    let excerpt = truncate_notes(&m.notes, 120);
                    println!("    {}", excerpt.bright_black());
                }
            }
        }

        SearchCommands::Events {
            place,
            event_type,
        } => {
            if place.is_none() && event_type.is_none() {
                println!(
                    "{}",
                    "Provide --place and/or --event-type to filter events.".yellow()
                );
                return Ok(());
            }

            let mut q = EventQuery::new();
            if let Some(ref p) = place {
                q = q.place_contains(p.as_str());
            }
            if let Some(ref et) = event_type {
                let parsed: kinforge_core::models::EventType = et
                    .parse()
                    .unwrap_or(kinforge_core::models::EventType::Other(et.clone()));
                q = q.of_type(parsed);
            }
            let events = q.run(&app.db)?;
            if events.is_empty() {
                println!("{}", "No matching events.".bright_black());
            } else {
                println!(
                    "{}\n",
                    format!("  {} event(s)  ", events.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for e in &events {
                    let person_name = app
                        .get_person(&e.person_id)
                        .map(|p| p.display_name())
                        .unwrap_or_else(|_| e.person_id.to_string());
                    let date_str = e
                        .date
                        .as_ref()
                        .map(|d| format!(" {}", d.to_string().yellow()))
                        .unwrap_or_default();
                    let place_str = e
                        .place_id
                        .as_ref()
                        .and_then(|pid| app.get_place(pid).ok())
                        .map(|pl| format!(" @ {}", pl.name.green()))
                        .unwrap_or_default();
                    println!(
                        "  {} {} {}{}{} {}",
                        e.id.to_string().bright_black(),
                        e.event_type.to_string().bright_cyan(),
                        "\u{2014}".bright_black(),
                        person_name.bold(),
                        date_str,
                        place_str
                    );
                }
            }
        }
    }
    Ok(())
}

/// Truncate a notes string to at most `max_chars` and add "…" if cut.
fn truncate_notes(s: &str, max_chars: usize) -> String {
    let single_line: String = s.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if single_line.chars().count() <= max_chars {
        single_line
    } else {
        let truncated: String = single_line.chars().take(max_chars).collect();
        format!("{}\u{2026}", truncated)
    }
}
