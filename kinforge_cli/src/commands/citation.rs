use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::{ConfidenceLevel, EventId, SourceId};

#[derive(Subcommand)]
pub enum CitationCommands {
    /// Add a citation linking a source to an event
    Add {
        /// Source ID
        #[arg(long)]
        source: String,
        /// Event ID
        #[arg(long)]
        event: String,
        /// Page reference
        #[arg(long)]
        page: Option<String>,
        /// Confidence level: unreliable, questionable, secondary, primary, direct
        #[arg(long, default_value = "secondary")]
        confidence: String,
        /// Notes
        #[arg(long)]
        notes: Option<String>,
    },
    /// List citations for an event
    List {
        /// Event ID
        event: String,
    },
}

pub fn handle(cmd: CitationCommands, app: &Application) -> Result<()> {
    match cmd {
        CitationCommands::Add { source, event, page, confidence, notes } => {
            let sid = SourceId::from_str(&source)?;
            let eid = EventId::from_str(&event)?;
            let conf: ConfidenceLevel = confidence.parse()?;

            let citation = app.add_citation(sid, eid, page.as_deref(), conf, notes.as_deref())?;
            println!("Added citation (ID: {})", citation.id);
        }
        CitationCommands::List { event } => {
            let eid = EventId::from_str(&event)?;
            let citations = app.db.list_citations_for_event(&eid)?;
            if citations.is_empty() {
                println!("No citations for this event.");
            } else {
                for c in &citations {
                    let page_str = c
                        .page
                        .as_deref()
                        .map(|p| format!(", p. {}", p))
                        .unwrap_or_default();
                    println!(
                        "  [{}] Source: {} | Confidence: {}{}",
                        c.id, c.source_id, c.confidence, page_str
                    );
                }
            }
        }
    }
    Ok(())
}
