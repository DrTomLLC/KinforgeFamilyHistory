use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_query::{PersonQuery, SourceQuery};

#[derive(Subcommand)]
pub enum SearchCommands {
    /// Search people by name
    People {
        query: String,
        #[arg(long)] sex: Option<String>,
    },
    /// Search sources by title or author
    Sources {
        query: String,
        #[arg(long)] from_year: Option<i32>,
        #[arg(long)] to_year: Option<i32>,
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
                println!("No people matching '{}'.", query);
            } else {
                println!("{} result(s):", results.len());
                for p in &results {
                    println!("  [{}] {} ({})", p.id, p.display_name(), p.sex);
                }
            }
        }

        SearchCommands::Sources { query, from_year, to_year } => {
            let mut q = SourceQuery::new().title_contains(&query);
            if let (Some(f), Some(t)) = (from_year, to_year) {
                q = q.year_range(f, t);
            }
            let results = q.run(&app.db)?;
            if results.is_empty() {
                println!("No sources matching '{}'.", query);
            } else {
                println!("{} result(s):", results.len());
                for s in &results {
                    let year = s.year.map(|y| format!(" ({})", y)).unwrap_or_default();
                    println!("  [{}] {}{}", s.id, s.title, year);
                }
            }
        }
    }
    Ok(())
}
