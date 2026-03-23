use anyhow::Result;
use clap::Subcommand;
use kinforge_app::Application;
use kinforge_core::models::{PersonId, RelationshipType};

#[derive(Subcommand)]
pub enum RelationshipCommands {
    /// Add a relationship between two people
    Add {
        /// First person ID
        #[arg(long)]
        person1: String,
        /// Relationship type: parent-child, spouse, sibling
        #[arg(long)]
        rel_type: String,
        /// Second person ID
        #[arg(long)]
        person2: String,
        /// Notes
        #[arg(long)]
        notes: Option<String>,
    },
    /// List relationships for a person
    List {
        /// Person ID
        person: String,
    },
}

pub fn handle(cmd: RelationshipCommands, app: &Application) -> Result<()> {
    match cmd {
        RelationshipCommands::Add { person1, rel_type, person2, notes } => {
            let p1 = PersonId::from_str(&person1)?;
            let p2 = PersonId::from_str(&person2)?;
            let rt: RelationshipType = rel_type.parse()?;

            let rel = app.add_relationship(rt, p1, p2, notes.as_deref())?;
            println!("Added relationship: {} (ID: {})", rel.rel_type, rel.id);
        }
        RelationshipCommands::List { person } => {
            let pid = PersonId::from_str(&person)?;
            let rels = app.list_relationships_for_person(&pid)?;
            if rels.is_empty() {
                println!("No relationships for this person.");
            } else {
                for r in &rels {
                    println!(
                        "  [{}] {} -- {} -- {}",
                        r.id, r.person1_id, r.rel_type, r.person2_id
                    );
                }
            }
        }
    }
    Ok(())
}
