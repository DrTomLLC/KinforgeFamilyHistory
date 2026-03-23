use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;

#[derive(Subcommand)]
pub enum SourceCommands {
    /// Add a source
    Add {
        /// Title
        #[arg(long)]
        title: String,
        /// Author
        #[arg(long)]
        author: Option<String>,
        /// Publication info
        #[arg(long)]
        publication: Option<String>,
        /// Year
        #[arg(long)]
        year: Option<i32>,
        /// Repository
        #[arg(long)]
        repository: Option<String>,
        /// Notes
        #[arg(long)]
        notes: Option<String>,
    },
    /// List all sources
    List,
}

pub fn handle(cmd: SourceCommands, app: &Application) -> Result<()> {
    match cmd {
        SourceCommands::Add { title, author, publication, year, repository, notes } => {
            let source = app.add_source(
                &title,
                author.as_deref(),
                publication.as_deref(),
                year,
                repository.as_deref(),
                notes.as_deref(),
            )?;
            println!("Added source: {} (ID: {})", source.title, source.id);
        }
        SourceCommands::List => {
            let sources = app.list_sources()?;
            if sources.is_empty() {
                println!("No sources in database.");
            } else {
                println!("{} source(s):", sources.len());
                for s in &sources {
                    let year_str = s
                        .year
                        .map(|y| format!(" ({})", y))
                        .unwrap_or_default();
                    println!("  [{}] {}{}", s.id, s.title, year_str);
                }
            }
        }
    }
    Ok(())
}
