use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_query::{EventQuery, PersonQuery, SourceQuery};

#[derive(Subcommand)]
pub enum SearchCommands {
    /// Search people by name and/or sex
    People {
        /// Match against any part of the full name
        #[arg(long)]
        name: Option<String>,
        /// Match given (first) name
        #[arg(long)]
        given: Option<String>,
        /// Match surname (last name)
        #[arg(long)]
        surname: Option<String>,
        /// Filter by sex: male, female, unknown
        #[arg(long)]
        sex: Option<String>,
    },
    /// Search sources by title, author, and/or year range
    Sources {
        /// Title fragment to search
        #[arg(long)]
        title: Option<String>,
        /// Author fragment to search
        #[arg(long)]
        author: Option<String>,
        /// Earliest year (use with --to-year)
        #[arg(long)]
        from_year: Option<i32>,
        /// Latest year (use with --from-year)
        #[arg(long)]
        to_year: Option<i32>,
    },
    /// Search person notes and event notes for a keyword
    Notes { query: String },
    /// Search events by place name, event type, and/or person
    Events {
        /// Place name fragment to search
        #[arg(long)]
        place: Option<String>,
        /// Event type filter (birth, death, marriage, etc.)
        #[arg(long)]
        event_type: Option<String>,
        /// Filter to a specific person (ID or short prefix)
        #[arg(long)]
        person: Option<String>,
        /// Earliest event year (inclusive)
        #[arg(long)]
        from_year: Option<i32>,
        /// Latest event year (inclusive)
        #[arg(long)]
        to_year: Option<i32>,
    },
    /// Search citations by source title fragment
    Citations {
        /// Source title fragment
        #[arg(long)]
        source: Option<String>,
    },
}

pub fn handle(cmd: SearchCommands, app: &Application) -> Result<()> {
    match cmd {
        SearchCommands::People { name, given, surname, sex } => {
            if name.is_none() && given.is_none() && surname.is_none() && sex.is_none() {
                println!(
                    "{}",
                    "Provide at least one filter: --name, --given, --surname, or --sex.".yellow()
                );
                return Ok(());
            }
            let mut q = PersonQuery::new();
            if let Some(ref n) = name {
                q = q.name_contains(n.as_str());
            }
            if let Some(ref g) = given {
                q = q.given_contains(g.as_str());
            }
            if let Some(ref s) = surname {
                q = q.surname_contains(s.as_str());
            }
            if let Some(s) = sex {
                q = q.sex(s.parse()?);
            }
            let results = q.run(&app.db)?;
            if results.is_empty() {
                println!("{}", "No matching people.".bright_black());
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

        SearchCommands::Sources { title, author, from_year, to_year } => {
            if title.is_none() && author.is_none() && from_year.is_none() {
                println!(
                    "{}",
                    "Provide at least one filter: --title, --author, or --from-year/--to-year."
                        .yellow()
                );
                return Ok(());
            }
            let mut q = SourceQuery::new();
            if let Some(ref t) = title {
                q = q.title_contains(t.as_str());
            }
            if let Some(ref a) = author {
                q = q.author_contains(a.as_str());
            }
            if let (Some(f), Some(t)) = (from_year, to_year) {
                q = q.year_range(f, t);
            }
            let results = q.run(&app.db)?;
            if results.is_empty() {
                println!("{}", "No matching sources.".bright_black());
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
                    let author_str = s
                        .author
                        .as_deref()
                        .map(|a| format!(" {}", format!("— {}", a).bright_black()))
                        .unwrap_or_default();
                    println!(
                        "  {} {}{}{}",
                        s.id.to_string().bright_black(),
                        s.title.bold(),
                        year,
                        author_str
                    );
                }
            }
        }

        SearchCommands::Notes { query } => {
            let results = app.search_notes(&query)?;
            if results.is_empty() {
                println!(
                    "{} \u{2018}{}\u{2019}",
                    "No notes matching".bright_black(),
                    query.yellow()
                );
            } else {
                println!(
                    "{}\n",
                    format!("  {} match(es)  ", results.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for m in &results {
                    println!(
                        "  {} {} {} {}",
                        m.kind.bright_cyan(),
                        m.id.bright_black(),
                        "\u{2014}".bright_black(),
                        m.label.bold()
                    );
                    let excerpt = truncate_notes(&m.notes, 120);
                    println!("    {}", excerpt.bright_black());
                }
            }
        }

        SearchCommands::Events { place, event_type, person, from_year, to_year } => {
            if place.is_none() && event_type.is_none() && person.is_none()
                && from_year.is_none() && to_year.is_none()
            {
                println!(
                    "{}",
                    "Provide --place, --event-type, --person, and/or --from-year/--to-year.".yellow()
                );
                return Ok(());
            }
            let mut q = EventQuery::new();
            if let Some(ref p) = place {
                q = q.place_contains(p.as_str());
            }
            if let Some(ref et) = event_type {
                let parsed: kinforge_core::models::EventType = et
                    .parse()
                    .unwrap_or(kinforge_core::models::EventType::Other(et.clone()));
                q = q.of_type(parsed);
            }
            if let Some(ref person_input) = person {
                let pid = app.resolve_person_id(person_input)?;
                q = q.for_person(pid);
            }
            if let Some(f) = from_year {
                q = q.from_year(f);
            }
            if let Some(t) = to_year {
                q = q.to_year(t);
            }
            let events = q.run(&app.db)?;
            if events.is_empty() {
                println!("{}", "No matching events.".bright_black());
            } else {
                println!(
                    "{}\n",
                    format!("  {} event(s)  ", events.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for e in &events {
                    let person_name = app
                        .get_person(&e.person_id)
                        .map(|p| p.display_name())
                        .unwrap_or_else(|_| e.person_id.to_string());
                    let date_str = e
                        .date
                        .as_ref()
                        .map(|d| format!(" {}", d.to_string().yellow()))
                        .unwrap_or_default();
                    let place_str = e
                        .place_id
                        .as_ref()
                        .and_then(|pid| app.get_place(pid).ok())
                        .map(|pl| format!(" @ {}", pl.name.green()))
                        .unwrap_or_default();
                    println!(
                        "  {} {} {}{}{} {}",
                        e.id.to_string().bright_black(),
                        e.event_type.to_string().bright_cyan(),
                        "\u{2014}".bright_black(),
                        person_name.bold(),
                        date_str,
                        place_str
                    );
                }
            }
        }

        SearchCommands::Citations { source } => {
            if source.is_none() {
                println!(
                    "{}",
                    "Provide --source <title fragment> to search citations.".yellow()
                );
                return Ok(());
            }
            let filter = source.as_deref().unwrap_or("").to_lowercase();
            let all_sources = app.list_sources()?;
            let matching_sources: Vec<_> = all_sources
                .iter()
                .filter(|s| s.title.to_lowercase().contains(&filter))
                .collect();

            if matching_sources.is_empty() {
                println!("{}", "No sources match that title fragment.".bright_black());
                return Ok(());
            }

            let mut total = 0usize;
            let mut out_lines: Vec<String> = Vec::new();

            for src in &matching_sources {
                let citations = app.list_citations_for_source(&src.id)?;
                if citations.is_empty() {
                    continue;
                }
                out_lines.push(format!(
                    "\n{} {}",
                    src.title.bold(),
                    src.id.to_string().bright_black()
                ));
                for c in &citations {
                    let event_label = app
                        .get_event(&c.event_id)
                        .ok()
                        .map(|e| {
                            let person_name = app
                                .get_person(&e.person_id)
                                .map(|p| p.display_name())
                                .unwrap_or_else(|_| e.person_id.to_string());
                            format!("{} \u{2014} {}", person_name, e.event_type)
                        })
                        .unwrap_or_else(|| "?".to_string());
                    let conf = fmt_confidence(&c.confidence);
                    let page_str = c
                        .page
                        .as_deref()
                        .map(|p| format!(" p.{}", p.yellow()))
                        .unwrap_or_default();
                    out_lines.push(format!(
                        "  {} {} {}{}",
                        c.id.to_string().bright_black(),
                        event_label.bold(),
                        conf,
                        page_str
                    ));
                    total += 1;
                }
            }

            if total == 0 {
                println!("{}", "No citations found for matching sources.".bright_black());
            } else {
                println!(
                    "{}\n",
                    format!("  {} citation(s)  ", total)
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for line in &out_lines {
                    println!("{}", line);
                }
            }
        }
    }
    Ok(())
}

fn fmt_confidence(conf: &kinforge_core::models::ConfidenceLevel) -> String {
    use kinforge_core::models::ConfidenceLevel;
    let s = conf.to_string();
    match conf {
        ConfidenceLevel::Direct => s.bright_green().bold().to_string(),
        ConfidenceLevel::Primary => s.green().to_string(),
        ConfidenceLevel::Secondary => s.yellow().to_string(),
        ConfidenceLevel::Questionable => s.red().to_string(),
        ConfidenceLevel::Unreliable => s.bright_red().bold().to_string(),
    }
}

fn truncate_notes(s: &str, max_chars: usize) -> String {
    let single_line: String = s.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if single_line.chars().count() <= max_chars {
        single_line
    } else {
        let truncated: String = single_line.chars().take(max_chars).collect();
        format!("{}\u{2026}", truncated)
    }
}
