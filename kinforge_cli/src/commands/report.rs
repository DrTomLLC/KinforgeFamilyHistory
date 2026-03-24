use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::PersonId;
use kinforge_reports::{ancestor_report, descendant_report, individual_report, people_list_report};
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
        #[arg(long, default_value = "4")] generations: u32,
    },
    /// Descendant chart
    Descendants {
        id: String,
        #[arg(long, default_value = "4")] generations: u32,
    },
    /// ASCII family tree (descendants)
    Tree {
        id: String,
        #[arg(long, default_value = "3")] depth: u32,
    },
}

pub fn handle(cmd: ReportCommands, app: &Application) -> Result<()> {
    match cmd {
        ReportCommands::Stats => {
            let s = app.stats()?;
            println!("Database statistics:");
            println!("  People:        {}", s.people);
            println!("  Events:        {}", s.events);
            println!("  Relationships: {}", s.relationships);
            println!("  Places:        {}", s.places);
            println!("  Sources:       {}", s.sources);
            println!("  Citations:     {}", s.citations);
            println!("  Database:      {}", app.config.database_path.display());
        }
        ReportCommands::People => {
            print!("{}", people_list_report(&app.db)?);
        }
        ReportCommands::Individual { id } => {
            let pid = PersonId::from_str(&id)?;
            print!("{}", individual_report(&app.db, &pid)?);
        }
        ReportCommands::Ancestors { id, generations } => {
            let pid = PersonId::from_str(&id)?;
            print!("{}", ancestor_report(&app.db, &pid, generations)?);
        }
        ReportCommands::Descendants { id, generations } => {
            let pid = PersonId::from_str(&id)?;
            print!("{}", descendant_report(&app.db, &pid, generations)?);
        }
        ReportCommands::Tree { id, depth } => {
            let pid = PersonId::from_str(&id)?;
            print!("{}", ascii_family_tree(&app.db, &pid, depth)?);
        }
    }
    Ok(())
}
