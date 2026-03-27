use anyhow::{bail, Result};
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_core::models::{EventDate, EventId, EventType, PersonId};

#[derive(Subcommand)]
pub enum EventCommands {
    /// Add an event for a person
    Add {
        #[arg(long)]
        person: String,
        #[arg(long)]
        event_type: String,
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: Option<String>,
        /// Date qualifier: exact (default), approximate, before, after, between
        #[arg(long, default_value = "exact")]
        qualifier: String,
        /// Second date for 'between' qualifier (YYYY-MM-DD)
        #[arg(long)]
        date2: Option<String>,
        /// Optional place name
        #[arg(long)]
        place: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    /// List events for a person
    List { person: String },
    /// Show a single event
    Show { id: String },
    /// Update an event's date or notes
    Update {
        id: String,
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: Option<String>,
        /// Date qualifier: exact (default), approximate, before, after, between
        #[arg(long, default_value = "exact")]
        qualifier: String,
        /// Second date for 'between' qualifier (YYYY-MM-DD)
        #[arg(long)]
        date2: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Delete an event
    Delete { id: String },
}

/// Parse a date string and qualifier into an EventDate.
/// Accepts YYYY-MM-DD or YYYY (year only, interpreted as Jan 1).
fn parse_event_date(date_str: &str, qualifier: &str, date2_str: Option<&str>) -> Result<EventDate> {
    let normalise = |s: &str| -> String {
        if s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()) {
            format!("{}-01-01", s)
        } else if s.len() == 7 && s.chars().nth(4) == Some('-') {
            format!("{}-01", s)
        } else {
            s.to_string()
        }
    };

    let d1 = normalise(date_str);
    let event_date = match qualifier.to_lowercase().as_str() {
        "exact" | "e" => EventDate::from_parts("exact", Some(&d1), None),
        "approximate" | "abt" | "about" | "approx" => {
            EventDate::from_parts("approximate", Some(&d1), None)
        }
        "before" | "bef" | "b" => EventDate::from_parts("before", Some(&d1), None),
        "after" | "aft" | "a" => EventDate::from_parts("after", Some(&d1), None),
        "between" | "bet" => {
            let d2 = date2_str
                .map(normalise)
                .ok_or_else(|| anyhow::anyhow!("--date2 is required for 'between' qualifier"))?;
            EventDate::from_parts("between", Some(&d1), Some(&d2))
        }
        other => bail!(
            "Unknown date qualifier '{}'. Use: exact, approximate, before, after, between",
            other
        ),
    };

    event_date.ok_or_else(|| {
        anyhow::anyhow!(
            "Could not parse date '{}'. Use YYYY, YYYY-MM, or YYYY-MM-DD format.",
            date_str
        )
    })
}

pub fn handle(cmd: EventCommands, app: &Application) -> Result<()> {
    match cmd {
        EventCommands::Add {
            person,
            event_type,
            date,
            qualifier,
            date2,
            place,
            notes,
        } => {
            let pid = PersonId::from_str(&person)?;
            let etype: EventType = event_type
                .parse()
                .unwrap_or(EventType::Other(event_type.clone()));
            let event_date = if let Some(ref d) = date {
                Some(parse_event_date(d, &qualifier, date2.as_deref())?)
            } else {
                None
            };
            let event =
                app.add_event(pid, etype, event_date, place.as_deref(), notes.as_deref())?;
            println!(
                "{} {} {}",
                "Added:".green().bold(),
                event.event_type.to_string().bright_cyan(),
                format!("({})", event.id).bright_black()
            );
        }

        EventCommands::List { person } => {
            let pid = PersonId::from_str(&person)?;
            let events = app.list_events_for_person(&pid)?;
            if events.is_empty() {
                println!("{}", "No events for this person.".bright_black());
            } else {
                println!(
                    "{}\n",
                    format!("  {} event(s)  ", events.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for e in &events {
                    let date_str = e.date.as_ref().map(|d| d.to_string()).unwrap_or_default();
                    let place_str = e
                        .place_id
                        .as_ref()
                        .and_then(|pid| app.get_place(pid).ok())
                        .map(|pl| format!(" @ {}", pl.name.green()))
                        .unwrap_or_default();
                    print!(
                        "  {} {}",
                        e.id.to_string().bright_black(),
                        e.event_type.to_string().bright_cyan()
                    );
                    if !date_str.is_empty() {
                        print!(": {}", date_str.yellow());
                    }
                    println!("{}", place_str);
                }
            }
        }

        EventCommands::Show { id } => {
            let eid = EventId::from_str(&id)?;
            let e = app.get_event(&eid)?;
            println!("{} {}", "ID:   ".cyan(), e.id.to_string().bright_black());
            println!("{} {}", "Type: ".cyan(), e.event_type.to_string().bright_cyan());
            if let Some(ref d) = e.date {
                println!("{} {}", "Date: ".cyan(), d.to_string().yellow());
            }
            if let Some(ref pid) = e.place_id {
                if let Ok(pl) = app.get_place(pid) {
                    println!("{} {}", "Place:".cyan(), pl.name.green());
                }
            }
            if let Some(ref n) = e.notes {
                println!("{} {}", "Notes:".cyan(), n);
            }
            let citations = app.list_citations_for_event(&eid)?;
            if !citations.is_empty() {
                println!("\n{}", format!("Citations ({}):", citations.len()).cyan().bold());
                for c in &citations {
                    let src_title = app
                        .get_source(&c.source_id)
                        .map(|s| s.title)
                        .unwrap_or_else(|_| "?".to_string());
                    let conf = format_confidence(&c.confidence);
                    println!(
                        "  {} {} {} {} {}",
                        c.id.to_string().bright_black(),
                        src_title.bold(),
                        "|".bright_black(),
                        c.page.as_deref().unwrap_or("no page").bright_black(),
                        format!("conf: {}", conf)
                    );
                }
            }
        }

        EventCommands::Update {
            id,
            date,
            qualifier,
            date2,
            notes,
        } => {
            let eid = EventId::from_str(&id)?;
            let mut event = app.get_event(&eid)?;
            if let Some(ref d) = date {
                event.date = Some(parse_event_date(d, &qualifier, date2.as_deref())?);
            }
            if let Some(n) = notes {
                event.notes = Some(n);
            }
            app.update_event(event)?;
            println!(
                "{} {}",
                "Updated:".green().bold(),
                id.bright_black()
            );
        }

        EventCommands::Delete { id } => {
            let eid = EventId::from_str(&id)?;
            app.delete_event(&eid)?;
            println!(
                "{} {}",
                "Deleted:".yellow().bold(),
                id.bright_black()
            );
        }
    }
    Ok(())
}

fn format_confidence(conf: &kinforge_core::models::ConfidenceLevel) -> String {
    use kinforge_core::models::ConfidenceLevel;
    let s = conf.to_string();
    match conf {
        ConfidenceLevel::Direct => s.bright_green().bold().to_string(),
        ConfidenceLevel::Primary => s.green().to_string(),
        ConfidenceLevel::Secondary => s.yellow().to_string(),
        ConfidenceLevel::Questionable => s.red().to_string(),
        ConfidenceLevel::Unreliable => s.bright_red().bold().to_string(),
    }
}
