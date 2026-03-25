use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::SourceId;

#[derive(Subcommand)]
pub enum SourceCommands {
    /// Add a source
    Add {
        #[arg(long)]
        title: String,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        publication: Option<String>,
        #[arg(long)]
        year: Option<i32>,
        #[arg(long)]
        repository: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    /// List all sources
    List,
    /// Show a source
    Show { id: String },
    /// Update a source
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        publication: Option<String>,
        #[arg(long)]
        year: Option<i32>,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Delete a source
    Delete { id: String },
}

pub fn handle(cmd: SourceCommands, app: &Application) -> Result<()> {
    match cmd {
        SourceCommands::Add {
            title,
            author,
            publication,
            year,
            repository,
            notes,
        } => {
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
                    let year_str = s.year.map(|y| format!(" ({})", y)).unwrap_or_default();
                    let author_str = s
                        .author
                        .as_deref()
                        .map(|a| format!(", {}", a))
                        .unwrap_or_default();
                    println!("  [{}] {}{}{}", s.id, s.title, year_str, author_str);
                }
            }
        }

        SourceCommands::Show { id } => {
            let sid = SourceId::from_str(&id)?;
            let s = app.get_source(&sid)?;
            println!("ID:          {}", s.id);
            println!("Title:       {}", s.title);
            if let Some(ref a) = s.author {
                println!("Author:      {}", a);
            }
            if let Some(ref p) = s.publication {
                println!("Publication: {}", p);
            }
            if let Some(y) = s.year {
                println!("Year:        {}", y);
            }
            if let Some(ref r) = s.repository {
                println!("Repository:  {}", r);
            }
            if let Some(ref n) = s.notes {
                println!("Notes:       {}", n);
            }
        }

        SourceCommands::Update {
            id,
            title,
            author,
            publication,
            year,
            notes,
        } => {
            let sid = SourceId::from_str(&id)?;
            let mut source = app.get_source(&sid)?;
            if let Some(t) = title {
                source.title = t;
            }
            if let Some(a) = author {
                source.author = Some(a);
            }
            if let Some(p) = publication {
                source.publication = Some(p);
            }
            if let Some(y) = year {
                source.year = Some(y);
            }
            if let Some(n) = notes {
                source.notes = Some(n);
            }
            app.update_source(source)?;
            println!("Updated source {}.", id);
        }

        SourceCommands::Delete { id } => {
            let sid = SourceId::from_str(&id)?;
            app.delete_source(&sid)?;
            println!("Deleted source {}.", id);
        }
    }
    Ok(())
}
