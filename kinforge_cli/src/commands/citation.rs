use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_core::models::{CitationId, ConfidenceLevel, EventId, SourceId};

#[derive(Subcommand)]
pub enum CitationCommands {
    /// Add a citation linking a source to an event
    Add {
        #[arg(long)]
        source: String,
        #[arg(long)]
        event: String,
        #[arg(long)]
        page: Option<String>,
        #[arg(long, default_value = "secondary")]
        confidence: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// List citations for an event
    List { event: String },
    /// Update a citation's page or confidence
    Update {
        id: String,
        #[arg(long)]
        page: Option<String>,
        #[arg(long)]
        confidence: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Delete a citation
    Delete { id: String },
}

fn fmt_confidence(conf: &ConfidenceLevel) -> String {
    let s = conf.to_string();
    match conf {
        ConfidenceLevel::Direct => s.bright_green().bold().to_string(),
        ConfidenceLevel::Primary => s.green().to_string(),
        ConfidenceLevel::Secondary => s.yellow().to_string(),
        ConfidenceLevel::Questionable => s.red().to_string(),
        ConfidenceLevel::Unreliable => s.bright_red().bold().to_string(),
    }
}

pub fn handle(cmd: CitationCommands, app: &Application) -> Result<()> {
    match cmd {
        CitationCommands::Add {
            source,
            event,
            page,
            confidence,
            notes,
        } => {
            let sid = SourceId::from_str(&source)?;
            let eid = EventId::from_str(&event)?;
            let conf: ConfidenceLevel = confidence.parse()?;
            let citation = app.add_citation(sid, eid, page.as_deref(), conf, notes.as_deref())?;
            println!(
                "{} {}",
                "Added citation:".green().bold(),
                citation.id.to_string().bright_black()
            );
        }

        CitationCommands::List { event } => {
            let eid = EventId::from_str(&event)?;
            let citations = app.list_citations_for_event(&eid)?;
            if citations.is_empty() {
                println!("{}", "No citations for this event.".bright_black());
            } else {
                println!(
                    "{}\n",
                    format!("  {} citation(s)  ", citations.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for c in &citations {
                    let src_title = app
                        .get_source(&c.source_id)
                        .map(|s| s.title)
                        .unwrap_or_else(|_| "?".to_string());
                    println!(
                        "  {} {} {} {} {}",
                        c.id.to_string().bright_black(),
                        src_title.bold(),
                        "|".bright_black(),
                        c.page.as_deref().unwrap_or("no page").bright_black(),
                        format!("conf: {}", fmt_confidence(&c.confidence))
                    );
                }
            }
        }

        CitationCommands::Update {
            id,
            page,
            confidence,
            notes,
        } => {
            let cid = CitationId::from_str(&id)?;
            let mut citation = app.get_citation(&cid)?;
            if let Some(p) = page {
                citation.page = Some(p);
            }
            if let Some(c) = confidence {
                citation.confidence = c.parse()?;
            }
            if let Some(n) = notes {
                citation.notes = Some(n);
            }
            app.update_citation(citation)?;
            println!(
                "{} {}",
                "Updated:".green().bold(),
                id.bright_black()
            );
        }

        CitationCommands::Delete { id } => {
            let cid = CitationId::from_str(&id)?;
            app.delete_citation(&cid)?;
            println!(
                "{} {}",
                "Deleted:".yellow().bold(),
                id.bright_black()
            );
        }
    }
    Ok(())
}
