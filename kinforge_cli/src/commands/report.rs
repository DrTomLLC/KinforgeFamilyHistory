use anyhow::Result;
use chrono::Datelike;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_core::models::{EventDate, EventType};
use kinforge_reports::{
    ancestor_report, birthdays_report, descendant_report, family_group_sheet,
    global_timeline_report, individual_report, narrative_report, people_list_report,
    places_report, sources_report, summary_report, timeline_report,
};
use kinforge_viz::{ascii_ancestor_tree, ascii_family_tree};
use std::collections::HashMap;

#[derive(Subcommand)]
pub enum ReportCommands {
    /// Database statistics
    Stats {
        /// Show detailed breakdown (decade histogram + top surnames)
        #[arg(long)]
        detailed: bool,
    },
    /// List all people
    People,
    /// Full individual summary
    Individual { id: String },
    /// Ancestor chart
    Ancestors {
        id: String,
        #[arg(long, default_value = "4")]
        generations: u32,
    },
    /// Descendant chart
    Descendants {
        id: String,
        #[arg(long, default_value = "4")]
        generations: u32,
    },
    /// ASCII family tree (descendants)
    Tree {
        id: String,
        #[arg(long, default_value = "3")]
        depth: u32,
    },
    /// Family Group Sheet (subject + spouse(s) + children)
    Family { id: String },
    /// Chronological timeline of all events for a person
    Timeline { id: String },
    /// ASCII ancestor tree (parents, grandparents, …)
    AncestorTree {
        id: String,
        #[arg(long, default_value = "3")]
        depth: u32,
    },
    /// All sources with citation counts
    Sources,
    /// Prose narrative biography for a person
    Narrative { id: String },
    /// All places with event counts, sorted by popularity
    Places,
    /// Compact database overview: counts, completeness, top surnames and event types
    Summary,
    /// Shortest relationship path between two people
    Path {
        /// Starting person (ID or prefix)
        #[arg(long)]
        from: String,
        /// Ending person (ID or prefix)
        #[arg(long)]
        to: String,
    },
    /// Chronological timeline of all events across all people
    GlobalTimeline {
        /// Limit to this many events (default: 200)
        #[arg(long, default_value = "200")]
        limit: usize,
    },
    /// Annual birthday reference — all people with known birth month and day, sorted by month/day
    Birthdays,
}

pub fn handle(cmd: ReportCommands, app: &Application) -> Result<()> {
    match cmd {
        ReportCommands::Stats { detailed } => {
            let s = app.stats()?;
            println!(
                "{}\n",
                "  Database Statistics  ".bold().bright_cyan().on_black()
            );
            let rows: &[(&str, String)] = &[
                ("People        ", s.people.to_string()),
                ("Events        ", s.events.to_string()),
                ("Relationships ", s.relationships.to_string()),
                ("Places        ", s.places.to_string()),
                ("Sources       ", s.sources.to_string()),
                ("Citations     ", s.citations.to_string()),
            ];
            for (label, value) in rows {
                println!(
                    "  {} {}",
                    label.cyan(),
                    value.bold().yellow()
                );
            }
            println!(
                "\n  {} {}",
                "Database:".cyan(),
                app.config.database_path.display().to_string().bright_black()
            );

            if detailed {
                // ── Birth decade histogram ────────────────────────────────────
                let people = app.list_people()?;
                let mut decade_counts: HashMap<i32, u32> = HashMap::new();
                let mut surname_counts: HashMap<String, u32> = HashMap::new();

                for p in &people {
                    // Surname tally
                    if let Some(sn) = p.names.first().and_then(|n| n.surname.as_deref()) {
                        if !sn.is_empty() {
                            *surname_counts.entry(sn.to_string()).or_insert(0) += 1;
                        }
                    }

                    // Birth decade
                    let events = app.list_events_for_person(&p.id).unwrap_or_default();
                    if let Some(birth_year) = events
                        .iter()
                        .find(|e| matches!(e.event_type, EventType::Birth))
                        .and_then(|e| e.date.as_ref())
                        .and_then(|d| match d {
                            EventDate::Exact(nd) | EventDate::Approximate(nd) => Some(nd.year()),
                            _ => None,
                        })
                    {
                        let decade = (birth_year / 10) * 10;
                        *decade_counts.entry(decade).or_insert(0) += 1;
                    }
                }

                if !decade_counts.is_empty() {
                    println!("\n  {}", "Birth Decade Histogram:".bold().cyan());
                    let mut decades: Vec<i32> = decade_counts.keys().cloned().collect();
                    decades.sort_unstable();
                    let max_count = *decade_counts.values().max().unwrap_or(&1);
                    for decade in &decades {
                        let count = decade_counts[decade];
                        let bar_len = (count as usize * 30) / max_count as usize;
                        let bar = "\u{2588}".repeat(bar_len);
                        println!(
                            "  {:>5}s  {} {}",
                            decade,
                            bar.yellow(),
                            count.to_string().bright_black()
                        );
                    }
                }

                if !surname_counts.is_empty() {
                    println!("\n  {}", "Top Surnames:".bold().cyan());
                    let mut surnames: Vec<(&String, &u32)> = surname_counts.iter().collect();
                    surnames.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
                    for (surname, count) in surnames.iter().take(10) {
                        println!(
                            "  {:>4}  {}",
                            count.to_string().yellow().bold(),
                            surname.bold()
                        );
                    }
                }
            }
        }
        ReportCommands::People => {
            print!("{}", people_list_report(app.database())?);
        }
        ReportCommands::Individual { id } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", individual_report(app.database(), &pid)?);
        }
        ReportCommands::Ancestors { id, generations } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", ancestor_report(app.database(), &pid, generations)?);
        }
        ReportCommands::Descendants { id, generations } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", descendant_report(app.database(), &pid, generations)?);
        }
        ReportCommands::Tree { id, depth } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", ascii_family_tree(app.database(), &pid, depth)?);
        }
        ReportCommands::Family { id } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", family_group_sheet(app.database(), &pid)?);
        }
        ReportCommands::Timeline { id } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", timeline_report(app.database(), &pid)?);
        }
        ReportCommands::AncestorTree { id, depth } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", ascii_ancestor_tree(app.database(), &pid, depth)?);
        }
        ReportCommands::Sources => {
            print!("{}", sources_report(app.database())?);
        }
        ReportCommands::Narrative { id } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", narrative_report(app.database(), &pid)?);
        }
        ReportCommands::Places => {
            print!("{}", places_report(app.database())?);
        }
        ReportCommands::Summary => {
            print!("{}", summary_report(app.database())?);
        }
        ReportCommands::Path { from, to } => {
            let from_id = app.resolve_person_id(&from)?;
            let to_id = app.resolve_person_id(&to)?;
            match app.find_relationship_path(&from_id, &to_id)? {
                None => {
                    println!("{}", "No relationship path found between these two people.".bright_black());
                }
                Some(path) => {
                    let from_name = app.get_person(&from_id).map(|p| p.display_name()).unwrap_or_default();
                    let to_name = app.get_person(&to_id).map(|p| p.display_name()).unwrap_or_default();
                    println!(
                        "{}\n",
                        format!("  Path: {} → {}  ", from_name, to_name)
                            .bold().bright_cyan().on_black()
                    );
                    let hops = path.steps.len().saturating_sub(1);
                    for line in path.describe() {
                        println!("  {}", line.bold());
                    }
                    println!(
                        "\n  {} {}",
                        "Degrees of separation:".cyan(),
                        hops.to_string().yellow().bold()
                    );
                }
            }
        }
        ReportCommands::GlobalTimeline { limit } => {
            print!("{}", global_timeline_report(app.database(), limit)?);
        }
        ReportCommands::Birthdays => {
            print!("{}", birthdays_report(app.database())?);
        }
    }
    Ok(())
}
