use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::Sex;

#[derive(Subcommand)]
pub enum PersonCommands {
    /// Add a new person
    Add {
        /// Given (first) name
        #[arg(long)]
        given: Option<String>,
        /// Surname (last name)
        #[arg(long)]
        surname: Option<String>,
        /// Sex: male, female, or unknown (default: unknown)
        #[arg(long, default_value = "unknown")]
        sex: String,
        /// Notes
        #[arg(long)]
        notes: Option<String>,
    },
    /// List all people
    List,
    /// Show a person's details
    Show {
        /// Person ID
        id: String,
    },
    /// Delete a person
    Delete {
        /// Person ID
        id: String,
    },
}

pub fn handle(cmd: PersonCommands, app: &Application) -> Result<()> {
    match cmd {
        PersonCommands::Add { given, surname, sex, notes } => {
            let sex_val: Sex = sex.parse()?;
            let person = app.add_person(
                given.as_deref(),
                surname.as_deref(),
                sex_val,
                notes.as_deref(),
            )?;
            println!("Added person: {} (ID: {})", person.display_name(), person.id);
        }
        PersonCommands::List => {
            let people = app.list_people()?;
            if people.is_empty() {
                println!("No people in database.");
            } else {
                println!("{} person(s):", people.len());
                for p in &people {
                    println!("  [{}] {} ({})", p.id, p.display_name(), p.sex);
                }
            }
        }
        PersonCommands::Show { id } => {
            use kinforge_core::models::PersonId;
            use kinforge_reports::individual_report;
            let pid = PersonId::from_str(&id)?;
            let report = individual_report(&app.db, &pid)?;
            print!("{}", report);
        }
        PersonCommands::Delete { id } => {
            use kinforge_core::models::PersonId;
            let pid = PersonId::from_str(&id)?;
            app.db.delete_person(&pid)?;
            println!("Deleted person {}", id);
        }
    }
    Ok(())
}
