use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::PlaceId;

#[derive(Subcommand)]
pub enum PlaceCommands {
    /// Add a place
    Add {
        #[arg(long)] name: String,
        #[arg(long)] latitude: Option<f64>,
        #[arg(long)] longitude: Option<f64>,
    },
    /// List all places
    List,
    /// Show a place
    Show { id: String },
    /// Update a place's name or coordinates
    Update {
        id: String,
        #[arg(long)] name: Option<String>,
        #[arg(long)] latitude: Option<f64>,
        #[arg(long)] longitude: Option<f64>,
    },
    /// Delete a place
    Delete { id: String },
}

pub fn handle(cmd: PlaceCommands, app: &Application) -> Result<()> {
    match cmd {
        PlaceCommands::Add { name, latitude, longitude } => {
            let place = app.add_place(&name, latitude, longitude)?;
            println!("Added place: {} (ID: {})", place.name, place.id);
        }

        PlaceCommands::List => {
            let places = app.list_places()?;
            if places.is_empty() {
                println!("No places in database.");
            } else {
                println!("{} place(s):", places.len());
                for p in &places {
                    let coords = match (p.latitude, p.longitude) {
                        (Some(lat), Some(lon)) => format!(" ({:.4}, {:.4})", lat, lon),
                        _ => String::new(),
                    };
                    println!("  [{}] {}{}", p.id, p.name, coords);
                }
            }
        }

        PlaceCommands::Show { id } => {
            let pid = PlaceId::from_str(&id)?;
            let p = app.db.get_place(&pid)?;
            println!("ID:   {}", p.id);
            println!("Name: {}", p.name);
            if let Some(lat) = p.latitude { println!("Lat:  {}", lat); }
            if let Some(lon) = p.longitude { println!("Lon:  {}", lon); }
        }

        PlaceCommands::Update { id, name, latitude, longitude } => {
            let pid = PlaceId::from_str(&id)?;
            let mut place = app.db.get_place(&pid)?;
            if let Some(n) = name { place.name = n; }
            if let Some(lat) = latitude { place.latitude = Some(lat); }
            if let Some(lon) = longitude { place.longitude = Some(lon); }
            app.update_place(place)?;
            println!("Updated place {}.", id);
        }

        PlaceCommands::Delete { id } => {
            let pid = PlaceId::from_str(&id)?;
            app.delete_place(&pid)?;
            println!("Deleted place {}.", id);
        }
    }
    Ok(())
}
