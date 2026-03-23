use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::{EventDate, EventType, PersonId};

#[derive(Subcommand)]
pub enum EventCommands {
    /// Add an event for a person
    Add {
        /// Person ID
        #[arg(long)]
        person: String,
        /// Event type (birth, death, marriage, burial, census, ...)
        #[arg(long)]
        event_type: String,
        /// Date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
        /// Place name
        #[arg(long)]
        place: Option<String>,
        /// Notes
        #[arg(long)]
        notes: Option<String>,
    },
    /// List events for a person
    List {
        /// Person ID
        person: String,
    },
}

pub fn handle(cmd: EventCommands, app: &Application) -> Result<()> {
    match cmd {
        EventCommands::Add { person, event_type, date, place, notes } => {
            let pid = PersonId::from_str(&person)?;
            let etype: EventType = event_type.parse().unwrap_or(EventType::Other(event_type.clone()));

            let event_date = date.as_deref().and_then(|d| {
                EventDate::from_parts("exact", Some(d), None)
            });

            let event = app.add_event(
                pid,
                etype,
                event_date,
                place.as_deref(),
                notes.as_deref(),
            )?;
            println!("Added event: {} (ID: {})", event.event_type, event.id);
        }
        EventCommands::List { person } => {
            let pid = PersonId::from_str(&person)?;
            let events = app.list_events_for_person(&pid)?;
            if events.is_empty() {
                println!("No events for this person.");
            } else {
                for e in &events {
                    let date_str = e
                        .date
                        .as_ref()
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "no date".to_string());
                    println!("  [{}] {} - {}", e.id, e.event_type, date_str);
                }
            }
        }
    }
    Ok(())
}
