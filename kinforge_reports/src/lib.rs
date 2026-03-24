use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;

/// Generate a plain-text individual summary report.
pub fn individual_report(db: &Database, person_id: &PersonId) -> KinforgeResult<String> {
    let person = db.get_person(person_id)?;
    let events = db.list_events_for_person(person_id)?;
    let relationships = db.list_relationships_for_person(person_id)?;

    let mut out = String::new();

    out.push_str(&format!("=== {} ===\n", person.display_name()));
    out.push_str(&format!("ID: {}\n", person.id));
    out.push_str(&format!("Sex: {}\n", person.sex));

    if person.names.len() > 1 {
        out.push_str("Names:\n");
        for name in &person.names {
            out.push_str(&format!("  {} ({})\n", name.full_name(), name.name_type));
        }
    }

    if let Some(ref notes) = person.notes {
        out.push_str(&format!("Notes: {}\n", notes));
    }

    if !events.is_empty() {
        out.push_str("\nEvents:\n");
        for event in &events {
            out.push_str(&format!("  {}", event.event_type));
            if let Some(ref date) = event.date {
                out.push_str(&format!(": {}", date));
            }
            if let Some(ref place_id) = event.place_id {
                if let Ok(place) = db.get_place(place_id) {
                    out.push_str(&format!(" at {}", place.name));
                }
            }
            if let Some(ref notes) = event.notes {
                out.push_str(&format!(" [{notes}]"));
            }
            out.push('\n');
        }
    }

    if !relationships.is_empty() {
        out.push_str("\nRelationships:\n");
        for rel in &relationships {
            let other_id = if rel.person1_id == *person_id {
                &rel.person2_id
            } else {
                &rel.person1_id
            };
            if let Ok(other) = db.get_person(other_id) {
                let role = describe_relationship(&rel.rel_type, person_id, &rel.person1_id);
                out.push_str(&format!(
                    "  {} {} ({})\n",
                    role,
                    other.display_name(),
                    other.id
                ));
            }
        }
    }

    Ok(out)
}

fn describe_relationship(
    rel_type: &RelationshipType,
    subject: &PersonId,
    person1: &PersonId,
) -> &'static str {
    match rel_type {
        RelationshipType::Spouse => "Spouse:",
        RelationshipType::Sibling => "Sibling:",
        RelationshipType::ParentChild => {
            if subject == person1 {
                "Parent of:"
            } else {
                "Child of:"
            }
        }
    }
}

/// Generate a plain-text list of all people.
pub fn people_list_report(db: &Database) -> KinforgeResult<String> {
    let people = db.list_people()?;
    let mut out = String::new();
    out.push_str(&format!("Total people: {}\n\n", people.len()));
    for person in &people {
        out.push_str(&format!(
            "  [{}] {} ({})\n",
            person.id,
            person.display_name(),
            person.sex
        ));
    }
    Ok(out)
}

/// Generate a plain-text ancestor report up to a given number of generations.
pub fn ancestor_report(
    db: &Database,
    person_id: &PersonId,
    generations: u32,
) -> KinforgeResult<String> {
    let mut out = String::new();
    out.push_str("=== Ancestor Report ===\n\n");
    build_ancestor_tree(db, person_id, 0, generations, &mut out)?;
    Ok(out)
}

fn build_ancestor_tree(
    db: &Database,
    person_id: &PersonId,
    depth: u32,
    max_depth: u32,
    out: &mut String,
) -> KinforgeResult<()> {
    if depth > max_depth {
        return Ok(());
    }
    let indent = "  ".repeat(depth as usize);
    let person = db.get_person(person_id)?;
    out.push_str(&format!("{}{}\n", indent, person.display_name()));

    if depth < max_depth {
        let rels = db.list_relationships_for_person(person_id)?;
        for rel in &rels {
            if rel.rel_type == RelationshipType::ParentChild && rel.person2_id == *person_id {
                build_ancestor_tree(db, &rel.person1_id, depth + 1, max_depth, out)?;
            }
        }
    }
    Ok(())
}

/// Generate a plain-text descendant report up to a given number of generations.
pub fn descendant_report(
    db: &Database,
    person_id: &PersonId,
    generations: u32,
) -> KinforgeResult<String> {
    let mut out = String::new();
    out.push_str("=== Descendant Report ===\n\n");
    build_descendant_tree(db, person_id, 0, generations, &mut out)?;
    Ok(out)
}

fn build_descendant_tree(
    db: &Database,
    person_id: &PersonId,
    depth: u32,
    max_depth: u32,
    out: &mut String,
) -> KinforgeResult<()> {
    if depth > max_depth {
        return Ok(());
    }
    let indent = "  ".repeat(depth as usize);
    let person = db.get_person(person_id)?;
    out.push_str(&format!("{}{}\n", indent, person.display_name()));

    if depth < max_depth {
        let rels = db.list_relationships_for_person(person_id)?;
        for rel in &rels {
            if rel.rel_type == RelationshipType::ParentChild && rel.person1_id == *person_id {
                build_descendant_tree(db, &rel.person2_id, depth + 1, max_depth, out)?;
            }
        }
    }
    Ok(())
}
