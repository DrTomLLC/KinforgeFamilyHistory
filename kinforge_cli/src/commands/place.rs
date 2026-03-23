use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::Place;

#[derive(Subcommand)]
pub enum PlaceCommands {
    /// Add a place
    Add {
        /// Place name
        #[arg(long)]
        name: String,
        /// Latitude
        #[arg(long)]
        latitude: Option<f64>,
        /// Longitude
        #[arg(long)]
        longitude: Option<f64>,
    },
    /// List all places
    List,
}

pub fn handle(cmd: PlaceCommands, app: &Application) -> Result<()> {
    match cmd {
        PlaceCommands::Add { name, latitude, longitude } => {
            let mut place = Place::new(name);
            place.latitude = latitude;
            place.longitude = longitude;
            app.db.insert_place(&place)?;
            println!("Added place: {} (ID: {})", place.name, place.id);
        }
        PlaceCommands::List => {
            let places = app.db.list_places()?;
            if places.is_empty() {
                println!("No places in database.");
            } else {
                for p in &places {
                    println!("  [{}] {}", p.id, p.name);
                }
            }
        }
    }
    Ok(())
}
