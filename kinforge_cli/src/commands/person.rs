use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_core::models::{NameType, RelationshipType, Sex};
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
    /// Edit an existing name entry in-place (by 0-based index shown in 'person show')
    UpdateName {
        id: String,
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
        #[arg(long, default_value = "0")]
        index: usize,
    },
    /// List this person's parents
    Parents { id: String },
    /// List this person's children
    Children { id: String },
    /// Record that someone is a parent of this person
    AddParent {
        /// The child's ID (full UUID or short prefix)
        id: String,
        /// The parent's ID (full UUID or short prefix)
        #[arg(long)]
        parent: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Record that someone is a child of this person
    AddChild {
        /// The parent's ID (full UUID or short prefix)
        id: String,
        /// The child's ID (full UUID or short prefix)
        #[arg(long)]
        child: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Record a spouse relationship with another person
    AddSpouse {
        id: String,
        #[arg(long)]
        spouse: String,
        #[arg(long)]
        notes: Option<String>,
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
            let pid = app.resolve_person_id(&id)?;
            let report = individual_report(&app.db, &pid)?;
            print!("{}", report);
        }

        PersonCommands::Update { id, sex, notes } => {
            let pid = app.resolve_person_id(&id)?;
            let mut person = app.get_person(&pid)?;
            if let Some(s) = sex {
                person.sex = s.parse()?;
            }
            if let Some(n) = notes {
                person.notes = Some(n);
            }
            app.update_person(person)?;
            println!("{} {}", "Updated:".green().bold(), id.bright_black());
        }

        PersonCommands::AddName {
            id,
            given,
            surname,
            name_type,
        } => {
            let pid = app.resolve_person_id(&id)?;
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
            let pid = app.resolve_person_id(&id)?;
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
            let pid = app.resolve_person_id(&id)?;
            let person = app.delete_name_from_person(&pid, index)?;
            println!(
                "{} name [{}] from {} {}",
                "Deleted:".yellow().bold(),
                index.to_string().yellow(),
                person.display_name().bold(),
                format!("— {} name(s) remain", person.names.len()).bright_black()
            );
        }

        PersonCommands::Parents { id } => {
            let pid = app.resolve_person_id(&id)?;
            let person = app.get_person(&pid)?;
            let rels = app.list_relationships_for_person(&pid)?;
            let parents: Vec<_> = rels
                .iter()
                .filter(|r| {
                    r.rel_type == RelationshipType::ParentChild && r.person2_id == pid
                })
                .collect();
            if parents.is_empty() {
                println!(
                    "{} {}",
                    "No parents recorded for".bright_black(),
                    person.display_name().bold()
                );
            } else {
                println!(
                    "{}\n",
                    format!("  Parents of {}  ", person.display_name())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for r in &parents {
                    if let Ok(parent) = app.get_person(&r.person1_id) {
                        println!(
                            "  {} {} {}",
                            r.person1_id.to_string().bright_black(),
                            parent.display_name().bold(),
                            format!("({})", parent.sex).bright_black()
                        );
                    }
                }
            }
        }

        PersonCommands::Children { id } => {
            let pid = app.resolve_person_id(&id)?;
            let person = app.get_person(&pid)?;
            let rels = app.list_relationships_for_person(&pid)?;
            let children: Vec<_> = rels
                .iter()
                .filter(|r| {
                    r.rel_type == RelationshipType::ParentChild && r.person1_id == pid
                })
                .collect();
            if children.is_empty() {
                println!(
                    "{} {}",
                    "No children recorded for".bright_black(),
                    person.display_name().bold()
                );
            } else {
                println!(
                    "{}\n",
                    format!("  Children of {}  ", person.display_name())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for r in &children {
                    if let Ok(child) = app.get_person(&r.person2_id) {
                        println!(
                            "  {} {} {}",
                            r.person2_id.to_string().bright_black(),
                            child.display_name().bold(),
                            format!("({})", child.sex).bright_black()
                        );
                    }
                }
            }
        }

        PersonCommands::AddParent { id, parent, notes } => {
            let child_id = app.resolve_person_id(&id)?;
            let parent_id = app.resolve_person_id(&parent)?;
            let child = app.get_person(&child_id)?;
            let par = app.get_person(&parent_id)?;
            app.add_parent(child_id, parent_id, notes.as_deref())?;
            println!(
                "{} {} {} {}",
                "Linked:".green().bold(),
                par.display_name().bold(),
                "\u{2192} parent of \u{2192}".bright_black(),
                child.display_name().bold()
            );
        }

        PersonCommands::AddChild { id, child, notes } => {
            let parent_id = app.resolve_person_id(&id)?;
            let child_id = app.resolve_person_id(&child)?;
            let parent = app.get_person(&parent_id)?;
            let ch = app.get_person(&child_id)?;
            app.add_child(parent_id, child_id, notes.as_deref())?;
            println!(
                "{} {} {} {}",
                "Linked:".green().bold(),
                parent.display_name().bold(),
                "\u{2192} parent of \u{2192}".bright_black(),
                ch.display_name().bold()
            );
        }

        PersonCommands::AddSpouse { id, spouse, notes } => {
            let p1 = app.resolve_person_id(&id)?;
            let p2 = app.resolve_person_id(&spouse)?;
            let n1 = app.get_person(&p1)?.display_name();
            let n2 = app.get_person(&p2)?.display_name();
            app.add_spouse(p1, p2, notes.as_deref())?;
            println!(
                "{} {} {} {}",
                "Linked:".green().bold(),
                n1.bold(),
                "\u{2194} spouse \u{2194}".bright_black(),
                n2.bold()
            );
        }

        PersonCommands::Delete { id } => {
            let pid = app.resolve_person_id(&id)?;
            app.delete_person(&pid)?;
            println!("{} {}", "Deleted:".yellow().bold(), id.bright_black());
        }
    }
    Ok(())
}
