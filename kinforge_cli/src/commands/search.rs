use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_query::search_people;

#[derive(Subcommand)]
pub enum SearchCommands {
    /// Search people by name
    People {
        /// Name fragment to search for
        query: String,
    },
}

pub fn handle(cmd: SearchCommands, app: &Application) -> Result<()> {
    match cmd {
        SearchCommands::People { query } => {
            let results = search_people(&app.db, &query)?;
            if results.is_empty() {
                println!("No results found for '{}'.", query);
            } else {
                println!("{} result(s):", results.len());
                for p in &results {
                    println!("  [{}] {} ({})", p.id, p.display_name(), p.sex);
                }
            }
        }
    }
    Ok(())
}
