use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_core::models::{PersonId, RelationshipId, RelationshipType};

#[derive(Subcommand)]
pub enum RelationshipCommands {
    /// Add a relationship between two people
    Add {
        #[arg(long)]
        person1: String,
        /// parent-child, spouse, sibling
        #[arg(long)]
        rel_type: String,
        #[arg(long)]
        person2: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Show a relationship's details
    Show { id: String },
    /// List relationships for a person
    List { person: String },
    /// Update a relationship's notes
    Update {
        id: String,
        #[arg(long)]
        notes: Option<String>,
    },
    /// Delete a relationship
    Delete { id: String },
}

pub fn handle(cmd: RelationshipCommands, app: &Application) -> Result<()> {
    match cmd {
        RelationshipCommands::Add {
            person1,
            rel_type,
            person2,
            notes,
        } => {
            let p1 = PersonId::from_str(&person1)?;
            let p2 = PersonId::from_str(&person2)?;
            let rt: RelationshipType = rel_type.parse()?;
            let rel = app.add_relationship(rt, p1, p2, notes.as_deref())?;
            println!(
                "{} {} {}",
                "Added:".green().bold(),
                rel.rel_type.to_string().bright_cyan(),
                format!("({})", rel.id).bright_black()
            );
        }

        RelationshipCommands::Show { id } => {
            let rid = RelationshipId::from_str(&id)?;
            let rel = app.get_relationship(&rid)?;
            let p1_name = app
                .get_person(&rel.person1_id)
                .map(|p| p.display_name())
                .unwrap_or_else(|_| rel.person1_id.to_string());
            let p2_name = app
                .get_person(&rel.person2_id)
                .map(|p| p.display_name())
                .unwrap_or_else(|_| rel.person2_id.to_string());
            println!("{} {}", "ID:     ".cyan(), rel.id.to_string().bright_black());
            println!("{} {}", "Type:   ".cyan(), rel.rel_type.to_string().bright_cyan());
            println!(
                "{} {} {}",
                "Person1:".cyan(),
                p1_name.bold(),
                format!("({})", rel.person1_id).bright_black()
            );
            println!(
                "{} {} {}",
                "Person2:".cyan(),
                p2_name.bold(),
                format!("({})", rel.person2_id).bright_black()
            );
            if let Some(ref n) = rel.notes {
                println!("{} {}", "Notes:  ".cyan(), n);
            }
        }

        RelationshipCommands::List { person } => {
            let pid = PersonId::from_str(&person)?;
            let rels = app.list_relationships_for_person(&pid)?;
            if rels.is_empty() {
                println!("{}", "No relationships for this person.".bright_black());
            } else {
                println!(
                    "{}\n",
                    format!("  {} relationship(s)  ", rels.len())
                        .bold()
                        .bright_cyan()
                        .on_black()
                );
                for r in &rels {
                    let other_id = if r.person1_id == pid {
                        &r.person2_id
                    } else {
                        &r.person1_id
                    };
                    let other_name = app
                        .get_person(other_id)
                        .map(|p| p.display_name())
                        .unwrap_or_else(|_| other_id.to_string());
                    let role = match &r.rel_type {
                        RelationshipType::Spouse => "Spouse:    ",
                        RelationshipType::Sibling => "Sibling:   ",
                        RelationshipType::ParentChild => {
                            if r.person1_id == pid {
                                "Parent of: "
                            } else {
                                "Child of:  "
                            }
                        }
                    };
                    let notes_str = r
                        .notes
                        .as_deref()
                        .map(|n| format!(" {}", format!("[{}]", n).bright_black()))
                        .unwrap_or_default();
                    println!(
                        "  {} {} {}{}",
                        role.cyan(),
                        other_name.bold(),
                        other_id.to_string().bright_black(),
                        notes_str
                    );
                }
            }
        }

        RelationshipCommands::Update { id, notes } => {
            let rid = RelationshipId::from_str(&id)?;
            let mut rel = app.get_relationship(&rid)?;
            if let Some(n) = notes {
                rel.notes = Some(n);
            }
            app.update_relationship(rel)?;
            println!(
                "{} {}",
                "Updated:".green().bold(),
                id.bright_black()
            );
        }

        RelationshipCommands::Delete { id } => {
            let rid = RelationshipId::from_str(&id)?;
            app.delete_relationship(&rid)?;
            println!(
                "{} {}",
                "Deleted:".yellow().bold(),
                id.bright_black()
            );
        }
    }
    Ok(())
}
