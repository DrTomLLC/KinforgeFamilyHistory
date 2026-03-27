use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
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
    /// Edit an existing name entry in-place (by 0-based index)
    UpdateName {
        id: String,
        /// 0-based index of the name to edit (use 'person show' to see indexes)
        #[arg(long, default_value = "0")]
        index: usize,
        #[arg(long)]
        given: Option<String>,
        #[arg(long)]
        surname: Option<String>,
        #[arg(long)]
        name_type: Option<String>,
    },
    /// Delete a name entry by index (cannot delete the only name)
    DeleteName {
        id: String,
        /// 0-based index of the name to remove
        #[arg(long, default_value = "0")]
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
                "{} {} {}",
                "Added:".green().bold(),
                person.display_name().bold(),
                format!("({})", person.id).bright_black()
            );
        }

        PersonCommands::List => {
            let people = app.list_people()?;
            if people.is_empty() {
                println!("{}", "No people in database.".bright_black());
            } else {
                println!(
                    "{}\n",
                    format!("  {} people  ", people.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for p in &people {
                    println!(
                        "  {} {} {}",
                        p.id.to_string().bright_black(),
                        p.display_name().bold(),
                        format!("({})", p.sex).bright_black()
                    );
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
            println!(
                "{} {}",
                "Updated:".green().bold(),
                id.bright_black()
            );
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
                "{} {} {} {}",
                "Added name to".green().bold(),
                person.display_name().bold(),
                "\u{2014}".bright_black(),
                format!("{} name(s) total", person.names.len()).bright_black()
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
            // Wrap given/surname: Some(v) means "set to v", None means "leave unchanged"
            let given_opt = given.map(|g| if g.is_empty() { None } else { Some(g) });
            let surname_opt = surname.map(|s| if s.is_empty() { None } else { Some(s) });
            let nt_opt = name_type.as_deref().map(|s| s.parse::<NameType>()).transpose()?;
            let person = app.update_name_on_person(&pid, index, given_opt, surname_opt, nt_opt)?;
            println!(
                "{} name [{}] on {} {}",
                "Updated:".green().bold(),
                index.to_string().yellow(),
                person.display_name().bold(),
                format!("({})", person.id).bright_black()
            );
        }

        PersonCommands::DeleteName { id, index } => {
            let pid = PersonId::from_str(&id)?;
            let person = app.delete_name_from_person(&pid, index)?;
            println!(
                "{} name [{}] from {} {}",
                "Deleted:".yellow().bold(),
                index.to_string().yellow(),
                person.display_name().bold(),
                format!("— {} name(s) remain", person.names.len()).bright_black()
            );
        }

        PersonCommands::Delete { id } => {
            let pid = PersonId::from_str(&id)?;
            app.delete_person(&pid)?;
            println!(
                "{} {}",
                "Deleted:".yellow().bold(),
                id.bright_black()
            );
        }
    }
    Ok(())
}
