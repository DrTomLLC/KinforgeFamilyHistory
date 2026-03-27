use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_reports::{
    ancestor_report, descendant_report, family_group_sheet, individual_report, people_list_report,
    timeline_report,
};
use kinforge_viz::ascii_family_tree;

#[derive(Subcommand)]
pub enum ReportCommands {
    /// Database statistics
    Stats,
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
}

pub fn handle(cmd: ReportCommands, app: &Application) -> Result<()> {
    match cmd {
        ReportCommands::Stats => {
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
        }
        ReportCommands::People => {
            print!("{}", people_list_report(&app.db)?);
        }
        ReportCommands::Individual { id } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", individual_report(&app.db, &pid)?);
        }
        ReportCommands::Ancestors { id, generations } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", ancestor_report(&app.db, &pid, generations)?);
        }
        ReportCommands::Descendants { id, generations } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", descendant_report(&app.db, &pid, generations)?);
        }
        ReportCommands::Tree { id, depth } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", ascii_family_tree(&app.db, &pid, depth)?);
        }
        ReportCommands::Family { id } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", family_group_sheet(&app.db, &pid)?);
        }
        ReportCommands::Timeline { id } => {
            let pid = app.resolve_person_id(&id)?;
            print!("{}", timeline_report(&app.db, &pid)?);
        }
    }
    Ok(())
}
