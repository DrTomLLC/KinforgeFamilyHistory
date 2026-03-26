use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::{NameType, PersonId, Sex};
use kinforge_reports::individual_report;

#[derive(Subcommand)]
pub enum PersonCommands {
    /// Add a new person
    Add {
        #[arg(long)]
        given: Option<String>,
        #[arg(long)]
        surname: Option<String>,
        #[arg(long, default_value = "unknown")]
        sex: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// List all people
    List,
    /// Show full details for a person
    Show { id: String },
    /// Update a person's sex or notes (use 'person add-name' to add names)
    Update {
        id: String,
        #[arg(long)]
        sex: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Add an alternative name to a person
    AddName {
        id: String,
        #[arg(long)]
        given: Option<String>,
        #[arg(long)]
        surname: Option<String>,
        #[arg(long, default_value = "birth")]
        name_type: String,
    },
    /// Edit an existing name entry by index (0 = primary name)
    UpdateName {
        id: String,
        /// Zero-based index of the name to edit (0 = primary/birth name)
        index: usize,
        #[arg(long)]
        given: Option<String>,
        #[arg(long)]
        surname: Option<String>,
        #[arg(long)]
        name_type: Option<String>,
    },
    /// Remove a name entry by index (cannot remove the last name)
    RemoveName {
        id: String,
        /// Zero-based index of the name to remove
        index: usize,
    },
    /// Delete a person (also deletes their events and relationships)
    Delete { id: String },
}

pub fn handle(cmd: PersonCommands, app: &Application) -> Result<()> {
    match cmd {
        PersonCommands::Add {
            given,
            surname,
            sex,
            notes,
        } => {
            let sex_val: Sex = sex.parse()?;
            let person = app.add_person(
                given.as_deref(),
                surname.as_deref(),
                sex_val,
                notes.as_deref(),
            )?;
            println!(
                "Added person: {} (ID: {})",
                person.display_name(),
                person.id
            );
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
            let pid = PersonId::from_str(&id)?;
            let report = individual_report(&app.db, &pid)?;
            print!("{}", report);
        }

        PersonCommands::Update { id, sex, notes } => {
            let pid = PersonId::from_str(&id)?;
            let mut person = app.get_person(&pid)?;
            if let Some(s) = sex {
                person.sex = s.parse()?;
            }
            if let Some(n) = notes {
                person.notes = Some(n);
            }
            app.update_person(person)?;
            println!("Updated person {}.", id);
        }

        PersonCommands::AddName {
            id,
            given,
            surname,
            name_type,
        } => {
            let pid = PersonId::from_str(&id)?;
            let nt: NameType = name_type.parse()?;
            let person = app.add_name_to_person(&pid, given.as_deref(), surname.as_deref(), nt)?;
            println!(
                "Added name to {} — now has {} name(s).",
                person.display_name(),
                person.names.len()
            );
        }

        PersonCommands::UpdateName {
            id,
            index,
            given,
            surname,
            name_type,
        } => {
            let pid = PersonId::from_str(&id)?;
            let mut person = app.get_person(&pid)?;
            if index >= person.names.len() {
                anyhow::bail!(
                    "Name index {} out of range — this person has {} name(s).",
                    index,
                    person.names.len()
                );
            }
            if let Some(g) = given {
                person.names[index].given = Some(g);
            }
            if let Some(s) = surname {
                person.names[index].surname = Some(s);
            }
            if let Some(nt) = name_type {
                person.names[index].name_type = nt.parse()?;
            }
            app.update_person(person)?;
            println!("Updated name[{}] for person {}.", index, id);
        }

        PersonCommands::RemoveName { id, index } => {
            let pid = PersonId::from_str(&id)?;
            let mut person = app.get_person(&pid)?;
            if person.names.len() <= 1 {
                anyhow::bail!("Cannot remove the only name entry for a person.");
            }
            if index >= person.names.len() {
                anyhow::bail!(
                    "Name index {} out of range — this person has {} name(s).",
                    index,
                    person.names.len()
                );
            }
            person.names.remove(index);
            app.update_person(person)?;
            println!("Removed name[{}] from person {}.", index, id);
        }

        PersonCommands::Delete { id } => {
            let pid = PersonId::from_str(&id)?;
            app.delete_person(&pid)?;
            println!("Deleted person {}.", id);
        }
    }
    Ok(())
}
