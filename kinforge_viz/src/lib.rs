use colored::Colorize;
use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;

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
