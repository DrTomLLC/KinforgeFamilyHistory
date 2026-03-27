use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;

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
            println!(
                "{} {} {}",
                "Added:".green().bold(),
                source.title.bold(),
                format!("({})", source.id).bright_black()
            );
        }

        SourceCommands::List => {
            let sources = app.list_sources()?;
            if sources.is_empty() {
                println!("{}", "No sources in database.".bright_black());
            } else {
                println!(
                    "{}\n",
                    format!("  {} source(s)  ", sources.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for s in &sources {
                    let year_str = s
                        .year
                        .map(|y| format!(" {}", format!("({})", y).yellow()))
                        .unwrap_or_default();
                    let author_str = s
                        .author
                        .as_deref()
                        .map(|a| format!(" {}", format!("— {}", a).bright_black()))
                        .unwrap_or_default();
                    println!(
                        "  {} {}{}{}",
                        s.id.to_string().bright_black(),
                        s.title.bold(),
                        year_str,
                        author_str
                    );
                }
            }
        }

        SourceCommands::Show { id } => {
            let sid = app.resolve_source_id(&id)?;
            let s = app.get_source(&sid)?;
            println!("{} {}", "ID:         ".cyan(), s.id.to_string().bright_black());
            println!("{} {}", "Title:      ".cyan(), s.title.bold());
            if let Some(ref a) = s.author {
                println!("{} {}", "Author:     ".cyan(), a);
            }
            if let Some(ref p) = s.publication {
                println!("{} {}", "Publication:".cyan(), p);
            }
            if let Some(y) = s.year {
                println!("{} {}", "Year:       ".cyan(), y.to_string().yellow());
            }
            if let Some(ref r) = s.repository {
                println!("{} {}", "Repository: ".cyan(), r);
            }
            if let Some(ref n) = s.notes {
                println!("{} {}", "Notes:      ".cyan(), n);
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
            let sid = app.resolve_source_id(&id)?;
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
            println!(
                "{} {}",
                "Updated:".green().bold(),
                id.bright_black()
            );
        }

        SourceCommands::Delete { id } => {
            let sid = app.resolve_source_id(&id)?;
            app.delete_source(&sid)?;
            println!(
                "{} {}",
                "Deleted:".yellow().bold(),
                id.bright_black()
            );
        }
    }
    Ok(())
}
