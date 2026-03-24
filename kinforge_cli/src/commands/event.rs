use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::{EventDate, EventId, EventType, PersonId};

#[derive(Subcommand)]
pub enum EventCommands {
    /// Add an event for a person
    Add {
        #[arg(long)] person: String,
        #[arg(long)] event_type: String,
        /// Exact date (YYYY-MM-DD)
        #[arg(long)] date: Option<String>,
        /// Optional place name
        #[arg(long)] place: Option<String>,
        #[arg(long)] notes: Option<String>,
    },
    /// List events for a person
    List { person: String },
    /// Show a single event
    Show { id: String },
    /// Update an event's date or notes
    Update {
        id: String,
        #[arg(long)] date: Option<String>,
        #[arg(long)] notes: Option<String>,
    },
    /// Delete an event
    Delete { id: String },
}

pub fn handle(cmd: EventCommands, app: &Application) -> Result<()> {
    match cmd {
        EventCommands::Add { person, event_type, date, place, notes } => {
            let pid = PersonId::from_str(&person)?;
            let etype: EventType = event_type.parse().unwrap_or(EventType::Other(event_type.clone()));
            let event_date = date.as_deref()
                .and_then(|d| EventDate::from_parts("exact", Some(d), None));
            let event = app.add_event(pid, etype, event_date, place.as_deref(), notes.as_deref())?;
            println!("Added event: {} (ID: {})", event.event_type, event.id);
        }

        EventCommands::List { person } => {
            let pid = PersonId::from_str(&person)?;
            let events = app.list_events_for_person(&pid)?;
            if events.is_empty() {
                println!("No events for this person.");
            } else {
                for e in &events {
                    let date_str = e.date.as_ref().map(|d| d.to_string()).unwrap_or_default();
                    let place_str = e.place_id.as_ref()
                        .and_then(|pid| app.db.get_place(pid).ok())
                        .map(|pl| format!(" @ {}", pl.name))
                        .unwrap_or_default();
                    println!("  [{}] {}{}{}", e.id, e.event_type,
                        if date_str.is_empty() { "".to_string() } else { format!(": {}", date_str) },
                        place_str);
                }
            }
        }

        EventCommands::Show { id } => {
            let eid = EventId::from_str(&id)?;
            let e = app.get_event(&eid)?;
            println!("ID:   {}", e.id);
            println!("Type: {}", e.event_type);
            if let Some(ref d) = e.date {
                println!("Date: {}", d);
            }
            if let Some(ref pid) = e.place_id {
                if let Ok(pl) = app.db.get_place(pid) {
                    println!("Place: {}", pl.name);
                }
            }
            if let Some(ref n) = e.notes {
                println!("Notes: {}", n);
            }
            // Show citations
            let citations = app.db.list_citations_for_event(&eid)?;
            if !citations.is_empty() {
                println!("\nCitations ({}):", citations.len());
                for c in &citations {
                    let src = app.db.get_source(&c.source_id);
                    let src_title = src.map(|s| s.title).unwrap_or_else(|_| "?".to_string());
                    println!("  [{}] {} | {} | conf: {}",
                        c.id, src_title,
                        c.page.as_deref().unwrap_or("no page"),
                        c.confidence);
                }
            }
        }

        EventCommands::Update { id, date, notes } => {
            let eid = EventId::from_str(&id)?;
            let mut event = app.get_event(&eid)?;
            if let Some(d) = date {
                event.date = EventDate::from_parts("exact", Some(&d), None);
            }
            if let Some(n) = notes {
                event.notes = Some(n);
            }
            app.update_event(event)?;
            println!("Updated event {}.", id);
        }

        EventCommands::Delete { id } => {
            let eid = EventId::from_str(&id)?;
            app.delete_event(&eid)?;
            println!("Deleted event {}.", id);
        }
    }
    Ok(())
}
