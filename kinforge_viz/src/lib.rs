use colored::Colorize;
use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;

/// Render an ASCII ancestor tree going up from a person.
///
/// Shows parents, grandparents, etc. up to `depth` generations.
/// Father is listed before mother at each level.
pub fn ascii_ancestor_tree(
    db: &Database,
    person_id: &PersonId,
    depth: u32,
) -> KinforgeResult<String> {
    let mut out = String::new();
    render_ancestor_node(db, person_id, depth, 0, "", true, &mut out)?;
    Ok(out)
}

fn render_ancestor_node(
    db: &Database,
    person_id: &PersonId,
    max_depth: u32,
    current_depth: u32,
    prefix: &str,
    is_last: bool,
    out: &mut String,
) -> KinforgeResult<()> {
    let person = db.get_person(person_id)?;

    let (connector, child_prefix): (&str, String) = if current_depth == 0 {
        ("", String::new())
    } else if is_last {
        ("└── ", format!("{}    ", prefix))
    } else {
        ("├── ", format!("{}│   ", prefix))
    };

    let life = birth_death_years(db, person_id);
    let name_str = person.display_name();

    if life.is_empty() {
        out.push_str(&format!(
            "{}{}{}\n",
            prefix,
            connector.bright_black(),
            name_str.bold()
        ));
    } else {
        out.push_str(&format!(
            "{}{}{} {}\n",
            prefix,
            connector.bright_black(),
            name_str.bold(),
            life.bright_black()
        ));
    }

    if current_depth < max_depth {
        let rels = db.list_relationships_for_person(person_id)?;
        let mut parents: Vec<&PersonId> = rels
            .iter()
            .filter(|r| {
                r.rel_type == RelationshipType::ParentChild && r.person2_id == *person_id
            })
            .map(|r| &r.person1_id)
            .collect();

        // Male first, then female, then unknown
        parents.sort_by_key(|pid| {
            db.get_person(pid)
                .map(|p| match p.sex {
                    Sex::Male => 0u8,
                    Sex::Female => 1u8,
                    Sex::Unknown => 2u8,
                })
                .unwrap_or(2)
        });

        let count = parents.len();
        for (i, parent_id) in parents.iter().enumerate() {
            let last = i == count - 1;
            render_ancestor_node(
                db,
                parent_id,
                max_depth,
                current_depth + 1,
                &child_prefix,
                last,
                out,
            )?;
        }
    }
    Ok(())
}

/// Render an ASCII family tree (descendants) centered on a person.
///
/// Uses box-drawing characters with proper ├── / └── connectors and tracks
/// the vertical continuation lines (│) so nested branches line up correctly.
pub fn ascii_family_tree(
    db: &Database,
    person_id: &PersonId,
    depth: u32,
) -> KinforgeResult<String> {
    let mut out = String::new();
    render_node(db, person_id, depth, 0, "", true, &mut out)?;
    Ok(out)
}

/// Recursive tree node renderer.
///
/// * `prefix`  — the continuation-line prefix accumulated from parent levels
///               (e.g. `"│   │   "`)
/// * `is_last` — whether this node is the last sibling at its level
fn render_node(
    db: &Database,
    person_id: &PersonId,
    max_depth: u32,
    current_depth: u32,
    prefix: &str,
    is_last: bool,
    out: &mut String,
) -> KinforgeResult<()> {
    let person = db.get_person(person_id)?;

    let (connector, child_prefix): (&str, String) = if current_depth == 0 {
        ("", String::new())
    } else if is_last {
        ("└── ", format!("{}    ", prefix))
    } else {
        ("├── ", format!("{}│   ", prefix))
    };

    let life = birth_death_years(db, person_id);
    let name_str = person.display_name();

    if life.is_empty() {
        out.push_str(&format!(
            "{}{}{}\n",
            prefix,
            connector.bright_black(),
            name_str.bold()
        ));
    } else {
        out.push_str(&format!(
            "{}{}{} {}\n",
            prefix,
            connector.bright_black(),
            name_str.bold(),
            life.bright_black()
        ));
    }

    if current_depth < max_depth {
        let rels = db.list_relationships_for_person(person_id)?;
        let children: Vec<&Relationship> = rels
            .iter()
            .filter(|r| r.rel_type == RelationshipType::ParentChild && r.person1_id == *person_id)
            .collect();

        let count = children.len();
        for (i, child_rel) in children.iter().enumerate() {
            let last = i == count - 1;
            render_node(
                db,
                &child_rel.person2_id,
                max_depth,
                current_depth + 1,
                &child_prefix,
                last,
                out,
            )?;
        }
    }
    Ok(())
}

/// Return a compact "(b.YYYY – d.YYYY)" string if birth/death data is available.
fn birth_death_years(db: &Database, person_id: &PersonId) -> String {
    let events = match db.list_events_for_person(person_id) {
        Ok(e) => e,
        Err(_) => return String::new(),
    };

    let birth_year = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Birth))
        .and_then(|e| e.date.as_ref())
        .and_then(year_from_date);

    let death_year = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Death))
        .and_then(|e| e.date.as_ref())
        .and_then(year_from_date);

    match (birth_year, death_year) {
        (Some(b), Some(d)) => format!("(b.{} \u{2013} d.{})", b, d),
        (Some(b), None) => format!("(b.{})", b),
        (None, Some(d)) => format!("(d.{})", d),
        (None, None) => String::new(),
    }
}

fn year_from_date(date: &EventDate) -> Option<String> {
    match date {
        EventDate::Exact(nd)
        | EventDate::Approximate(nd)
        | EventDate::Before(nd)
        | EventDate::After(nd) => Some(nd.format("%Y").to_string()),
        EventDate::Between(nd, _) => Some(nd.format("%Y").to_string()),
        EventDate::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kinforge_core::models::{NameType, Person, PersonName, Relationship, RelationshipType, Sex};
    use kinforge_storage::Database;

    fn make_person(db: &Database, given: &str, surname: &str, sex: Sex) -> Person {
        let mut p = Person::new(sex);
        p.names.push(PersonName {
            given: Some(given.to_string()),
            surname: Some(surname.to_string()),
            name_type: NameType::Birth,
            prefix: None,
            suffix: None,
        });
        db.insert_person(&p).unwrap();
        p
    }

    #[test]
    fn descendant_tree_renders_children() {
        let db = Database::open_in_memory().unwrap();
        let parent = make_person(&db, "Adam", "Jones", Sex::Male);
        let child = make_person(&db, "Beth", "Jones", Sex::Female);
        let rel = Relationship::new(RelationshipType::ParentChild, parent.id.clone(), child.id.clone());
        db.insert_relationship(&rel).unwrap();

        let tree = ascii_family_tree(&db, &parent.id, 2).unwrap();
        assert!(tree.contains("Adam Jones"));
        assert!(tree.contains("Beth Jones"));
        assert!(tree.contains("└──") || tree.contains("├──"));
    }

    #[test]
    fn ancestor_tree_renders_parents() {
        let db = Database::open_in_memory().unwrap();
        let grandpa = make_person(&db, "George", "Smith", Sex::Male);
        let father = make_person(&db, "Henry", "Smith", Sex::Male);
        let subject = make_person(&db, "John", "Smith", Sex::Male);

        let r1 = Relationship::new(RelationshipType::ParentChild, grandpa.id.clone(), father.id.clone());
        let r2 = Relationship::new(RelationshipType::ParentChild, father.id.clone(), subject.id.clone());
        db.insert_relationship(&r1).unwrap();
        db.insert_relationship(&r2).unwrap();

        let tree = ascii_ancestor_tree(&db, &subject.id, 3).unwrap();
        assert!(tree.contains("John Smith"));
        assert!(tree.contains("Henry Smith"));
        assert!(tree.contains("George Smith"));
    }
}
