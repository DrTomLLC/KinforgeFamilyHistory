use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::PersonId;
use kinforge_reports::{ancestor_report, descendant_report, individual_report, people_list_report};
use kinforge_viz::ascii_family_tree;

#[derive(Subcommand)]
pub enum ReportCommands {
    /// List all people (summary)
    People,
    /// Full individual report
    Individual {
        /// Person ID
        id: String,
    },
    /// Ancestor report
    Ancestors {
        /// Person ID
        id: String,
        /// Number of generations (default: 4)
        #[arg(long, default_value = "4")]
        generations: u32,
    },
    /// Descendant report
    Descendants {
        /// Person ID
        id: String,
        /// Number of generations (default: 4)
        #[arg(long, default_value = "4")]
        generations: u32,
    },
    /// ASCII family tree
    Tree {
        /// Person ID
        id: String,
        /// Depth (default: 3)
        #[arg(long, default_value = "3")]
        depth: u32,
    },
}

pub fn handle(cmd: ReportCommands, app: &Application) -> Result<()> {
    match cmd {
        ReportCommands::People => {
            let report = people_list_report(&app.db)?;
            print!("{}", report);
        }
        ReportCommands::Individual { id } => {
            let pid = PersonId::from_str(&id)?;
            let report = individual_report(&app.db, &pid)?;
            print!("{}", report);
        }
        ReportCommands::Ancestors { id, generations } => {
            let pid = PersonId::from_str(&id)?;
            let report = ancestor_report(&app.db, &pid, generations)?;
            print!("{}", report);
        }
        ReportCommands::Descendants { id, generations } => {
            let pid = PersonId::from_str(&id)?;
            let report = descendant_report(&app.db, &pid, generations)?;
            print!("{}", report);
        }
        ReportCommands::Tree { id, depth } => {
            let pid = PersonId::from_str(&id)?;
            let tree = ascii_family_tree(&app.db, &pid, depth)?;
            print!("{}", tree);
        }
    }
    Ok(())
}
