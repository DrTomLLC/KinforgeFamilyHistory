use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_query::{PersonQuery, SourceQuery};

#[derive(Subcommand)]
pub enum SearchCommands {
    /// Search people by name
    People {
        query: String,
        #[arg(long)]
        sex: Option<String>,
    },
    /// Search sources by title or author
    Sources {
        query: String,
        #[arg(long)]
        from_year: Option<i32>,
        #[arg(long)]
        to_year: Option<i32>,
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
                println!(
                    "{} {}{}{}",
                    "No people matching".bright_black(),
                    "\u{2018}".bright_black(),
                    query.yellow(),
                    "\u{2019}.".bright_black()
                );
            } else {
                println!(
                    "{}\n",
                    format!("  {} result(s)  ", results.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for p in &results {
                    println!(
                        "  {} {} {}",
                        p.id.to_string().bright_black(),
                        p.display_name().bold(),
                        format!("({})", p.sex).bright_black()
                    );
                }
            }
        }

        SearchCommands::Sources {
            query,
            from_year,
            to_year,
        } => {
            let mut q = SourceQuery::new().title_contains(&query);
            if let (Some(f), Some(t)) = (from_year, to_year) {
                q = q.year_range(f, t);
            }
            let results = q.run(&app.db)?;
            if results.is_empty() {
                println!(
                    "{} {}{}{}",
                    "No sources matching".bright_black(),
                    "\u{2018}".bright_black(),
                    query.yellow(),
                    "\u{2019}.".bright_black()
                );
            } else {
                println!(
                    "{}\n",
                    format!("  {} result(s)  ", results.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for s in &results {
                    let year = s
                        .year
                        .map(|y| format!(" {}", format!("({})", y).yellow()))
                        .unwrap_or_default();
                    println!(
                        "  {} {}{}",
                        s.id.to_string().bright_black(),
                        s.title.bold(),
                        year
                    );
                }
            }
        }
    }
    Ok(())
}
