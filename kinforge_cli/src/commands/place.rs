use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_core::models::PlaceId;

#[derive(Subcommand)]
pub enum PlaceCommands {
    /// Add a place
    Add {
        #[arg(long)]
        name: String,
        #[arg(long)]
        latitude: Option<f64>,
        #[arg(long)]
        longitude: Option<f64>,
        /// UUID of the parent place (e.g. a county that contains this town)
        #[arg(long)]
        parent: Option<String>,
    },
    /// List all places
    List,
    /// Show a place
    Show { id: String },
    /// Update a place's name, coordinates, or parent
    Update {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        latitude: Option<f64>,
        #[arg(long)]
        longitude: Option<f64>,
        /// Set or change the parent place UUID
        #[arg(long)]
        parent: Option<String>,
    },
    /// Delete a place
    Delete { id: String },
}

pub fn handle(cmd: PlaceCommands, app: &Application) -> Result<()> {
    match cmd {
        PlaceCommands::Add {
            name,
            latitude,
            longitude,
            parent,
        } => {
            let parent_id = parent.as_deref().map(PlaceId::from_str).transpose()?;
            let place = app.add_place(&name, latitude, longitude, parent_id)?;
            println!(
                "{} {} {}",
                "Added:".green().bold(),
                place.name.bold(),
                format!("({})", place.id).bright_black()
            );
        }

        PlaceCommands::List => {
            let places = app.list_places()?;
            if places.is_empty() {
                println!("{}", "No places in database.".bright_black());
            } else {
                println!(
                    "{}\n",
                    format!("  {} place(s)  ", places.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for p in &places {
                    let coords = match (p.latitude, p.longitude) {
                        (Some(lat), Some(lon)) => {
                            format!(" {}", format!("({:.4}, {:.4})", lat, lon).bright_black())
                        }
                        _ => String::new(),
                    };
                    let parent_str = p
                        .parent_id
                        .as_ref()
                        .and_then(|pid| app.get_place(pid).ok())
                        .map(|parent| {
                            format!(" {}", format!("in: {}", parent.name).bright_black())
                        })
                        .unwrap_or_default();
                    println!(
                        "  {} {}{}{}",
                        p.id.to_string().bright_black(),
                        p.name.bold(),
                        coords,
                        parent_str
                    );
                }
            }
        }

        PlaceCommands::Show { id } => {
            let pid = PlaceId::from_str(&id)?;
            let p = app.get_place(&pid)?;
            println!("{} {}", "ID:  ".cyan(), p.id.to_string().bright_black());
            println!("{} {}", "Name:".cyan(), p.name.bold());
            if let (Some(lat), Some(lon)) = (p.latitude, p.longitude) {
                println!(
                    "{} {}, {}",
                    "Coords:".cyan(),
                    lat.to_string().yellow(),
                    lon.to_string().yellow()
                );
            }
            if let Some(ref parent_id) = p.parent_id {
                let parent_name = app
                    .get_place(parent_id)
                    .map(|pp| pp.name)
                    .unwrap_or_else(|_| parent_id.to_string());
                println!(
                    "{} {} {}",
                    "Part of:".cyan(),
                    parent_name.bold(),
                    format!("({})", parent_id).bright_black()
                );
            }
        }

        PlaceCommands::Update {
            id,
            name,
            latitude,
            longitude,
            parent,
        } => {
            let pid = PlaceId::from_str(&id)?;
            let mut place = app.get_place(&pid)?;
            if let Some(n) = name {
                place.name = n;
            }
            if let Some(lat) = latitude {
                place.latitude = Some(lat);
            }
            if let Some(lon) = longitude {
                place.longitude = Some(lon);
            }
            if let Some(ref p) = parent {
                place.parent_id = Some(PlaceId::from_str(p)?);
            }
            app.update_place(place)?;
            println!(
                "{} {}",
                "Updated:".green().bold(),
                id.bright_black()
            );
        }

        PlaceCommands::Delete { id } => {
            let pid = PlaceId::from_str(&id)?;
            app.delete_place(&pid)?;
            println!(
                "{} {}",
                "Deleted:".yellow().bold(),
                id.bright_black()
            );
        }
    }
    Ok(())
}
