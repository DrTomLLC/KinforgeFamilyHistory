use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_core::models::{MediaEntityType, MediaType};

#[derive(Subcommand)]
pub enum MediaCommands {
    /// Add a new media record
    Add {
        /// Title or filename
        #[arg(long)]
        title: String,
        /// Media type: photo, document, audio, video, other
        #[arg(long, default_value = "other")]
        media_type: String,
        /// Local filesystem path
        #[arg(long)]
        path: Option<String>,
        /// Remote URL
        #[arg(long)]
        url: Option<String>,
        /// Description
        #[arg(long)]
        description: Option<String>,
        /// Date (free-form, e.g. "1920", "circa 1920")
        #[arg(long)]
        date: Option<String>,
    },
    /// Show a media record
    Show {
        /// Media ID or prefix
        id: String,
    },
    /// List all media records
    List,
    /// Update a media record
    Update {
        /// Media ID or prefix
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        media_type: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        date: Option<String>,
        /// Clear the path field
        #[arg(long)]
        clear_path: bool,
        /// Clear the URL field
        #[arg(long)]
        clear_url: bool,
    },
    /// Delete a media record (and all its links)
    Delete {
        /// Media ID or prefix
        id: String,
    },
    /// Attach a media record to a person, event, or source
    Attach {
        /// Media ID or prefix
        id: String,
        /// Entity type: person, event, source
        #[arg(long)]
        to: String,
        /// Entity ID or prefix
        #[arg(long)]
        entity: String,
    },
    /// Detach a media record from an entity (by link ID)
    Detach {
        /// Media link ID or prefix
        link_id: String,
    },
    /// List all media attached to a person
    ForPerson {
        /// Person ID or prefix
        id: String,
    },
    /// List all media attached to an event
    ForEvent {
        /// Event ID or prefix
        id: String,
    },
}

pub fn handle(cmd: MediaCommands, app: &Application) -> Result<()> {
    match cmd {
        MediaCommands::Add {
            title,
            media_type,
            path,
            url,
            description,
            date,
        } => {
            let mt: MediaType = media_type.parse()?;
            let m = app.add_media(
                &title,
                mt,
                path.as_deref(),
                url.as_deref(),
                description.as_deref(),
                date.as_deref(),
            )?;
            println!(
                "{} {} {}",
                "Added media".green(),
                m.id.as_str().bright_black(),
                m.title.bold()
            );
        }

        MediaCommands::Show { id } => {
            let mid = app.resolve_media_id(&id)?;
            let m = app.get_media(&mid)?;
            print_media(&m);
            let links = app.list_media_links_for_media(&mid)?;
            if !links.is_empty() {
                println!("  {}", "Attached to:".cyan());
                for link in &links {
                    println!(
                        "    {} {} ({})",
                        link.entity_type.to_string().yellow(),
                        link.entity_id.bright_black(),
                        link.id.as_str().bright_black()
                    );
                }
            }
        }

        MediaCommands::List => {
            let all = app.list_media()?;
            if all.is_empty() {
                println!("{}", "No media records.".bright_black());
                return Ok(());
            }
            println!("{}", "  Media Library  ".bold().bright_cyan().on_black());
            for m in &all {
                println!(
                    "  {} {} [{}]{}",
                    m.id.as_str()[..8].bright_black(),
                    m.title.bold(),
                    m.media_type.to_string().yellow(),
                    m.date
                        .as_deref()
                        .map(|d| format!(" {}", d.bright_black()))
                        .unwrap_or_default()
                );
            }
        }

        MediaCommands::Update {
            id,
            title,
            media_type,
            path,
            url,
            description,
            date,
            clear_path,
            clear_url,
        } => {
            let mid = app.resolve_media_id(&id)?;
            let mut m = app.get_media(&mid)?;
            if let Some(t) = title {
                m.title = t;
            }
            if let Some(mt) = media_type {
                m.media_type = mt.parse()?;
            }
            if clear_path {
                m.path = None;
            } else if let Some(p) = path {
                m.path = Some(p);
            }
            if clear_url {
                m.url = None;
            } else if let Some(u) = url {
                m.url = Some(u);
            }
            if let Some(d) = description {
                m.description = Some(d);
            }
            if let Some(dt) = date {
                m.date = Some(dt);
            }
            let updated = app.update_media(m)?;
            println!(
                "{} {}",
                "Updated media".green(),
                updated.title.bold()
            );
        }

        MediaCommands::Delete { id } => {
            let mid = app.resolve_media_id(&id)?;
            let m = app.get_media(&mid)?;
            app.delete_media(&mid)?;
            println!("{} {}", "Deleted media".red(), m.title.bold());
        }

        MediaCommands::Attach { id, to, entity } => {
            let mid = app.resolve_media_id(&id)?;
            let entity_type: MediaEntityType = to.parse()?;
            let entity_id = match entity_type {
                MediaEntityType::Person => app.resolve_person_id(&entity)?.as_str(),
                MediaEntityType::Event => app.resolve_event_id(&entity)?.as_str(),
                MediaEntityType::Source => app.resolve_source_id(&entity)?.as_str(),
            };
            let link = app.attach_media(&mid, entity_type, &entity_id)?;
            println!(
                "{} (link {})",
                "Attached".green(),
                link.id.as_str().bright_black()
            );
        }

        MediaCommands::Detach { link_id } => {
            // link IDs are full UUIDs; parse directly
            let lid = kinforge_core::models::MediaLinkId::from_str(&link_id)
                .map_err(|e| anyhow::anyhow!("Invalid link ID: {}", e))?;
            app.detach_media(&lid)?;
            println!("{}", "Detached".green());
        }

        MediaCommands::ForPerson { id } => {
            let pid = app.resolve_person_id(&id)?;
            let media = app.list_media_for_person(&pid)?;
            if media.is_empty() {
                println!("{}", "No media attached to this person.".bright_black());
            } else {
                for m in &media {
                    print_media(m);
                }
            }
        }

        MediaCommands::ForEvent { id } => {
            let eid = app.resolve_event_id(&id)?;
            let media = app.list_media_for_event(&eid)?;
            if media.is_empty() {
                println!("{}", "No media attached to this event.".bright_black());
            } else {
                for m in &media {
                    print_media(m);
                }
            }
        }
    }
    Ok(())
}

fn print_media(m: &kinforge_core::models::Media) {
    println!(
        "\n  {} {}  [{}]",
        "Media:".cyan(),
        m.title.bold(),
        m.media_type.to_string().yellow()
    );
    println!("  {} {}", "ID:".cyan(), m.id.as_str().bright_black());
    if let Some(ref p) = m.path {
        println!("  {} {}", "Path:".cyan(), p);
    }
    if let Some(ref u) = m.url {
        println!("  {} {}", "URL:".cyan(), u);
    }
    if let Some(ref d) = m.date {
        println!("  {} {}", "Date:".cyan(), d.yellow());
    }
    if let Some(ref desc) = m.description {
        println!("  {} {}", "Desc:".cyan(), desc);
    }
}

