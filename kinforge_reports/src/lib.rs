use colored::Colorize;
use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;

// ─── helpers ────────────────────────────────────────────────────────────────

fn fmt_id(id: &str) -> String {
    id.bright_black().to_string()
}

fn fmt_sex(sex: &Sex) -> String {
    match sex {
        Sex::Male => "male".bright_blue().to_string(),
        Sex::Female => "female".bright_magenta().to_string(),
        Sex::Unknown => "unknown".bright_black().to_string(),
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

// ─── individual report ──────────────────────────────────────────────────────

/// Generate a colored individual summary report.
pub fn individual_report(db: &Database, person_id: &PersonId) -> KinforgeResult<String> {
    let person = db.get_person(person_id)?;
    let events = db.list_events_for_person(person_id)?;
    let relationships = db.list_relationships_for_person(person_id)?;

    let mut out = String::new();

    // Header bar
    let header = format!("  {}  ", person.display_name());
    out.push_str(&format!("{}\n", header.bold().bright_cyan().on_black()));
    out.push_str(&format!("{}\n", "─".repeat(header.len()).bright_black()));

    // Core fields
    out.push_str(&format!(
        "{} {}\n",
        "ID: ".cyan(),
        fmt_id(&person.id.to_string())
    ));
    out.push_str(&format!("{} {}\n", "Sex:".cyan(), fmt_sex(&person.sex)));

    // Life span
    let birth = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Birth));
    let death = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Death));
    let born_str = birth
        .and_then(|e| e.date.as_ref())
        .map(|d| d.to_string())
        .unwrap_or_else(|| "?".to_string());
    let died_str = death
        .and_then(|e| e.date.as_ref())
        .map(|d| d.to_string())
        .unwrap_or_else(|| "living".to_string());
    out.push_str(&format!(
        "{} {} \u{2013} {}\n",
        "Life:".cyan(),
        born_str.yellow(),
        died_str.yellow()
    ));

    // Extra names
    if person.names.len() > 1 {
        out.push_str(&format!("\n{}\n", "Names:".cyan().bold()));
        for name in &person.names {
            out.push_str(&format!(
                "  {} {}\n",
                name.full_name().bold(),
                format!("({})", name.name_type).bright_black()
            ));
        }
    }

    // Notes
    if let Some(ref notes) = person.notes {
        out.push_str(&format!("\n{} {}\n", "Notes:".cyan(), notes));
    }

    // Events
    if !events.is_empty() {
        out.push_str(&format!("\n{}\n", "Events:".cyan().bold()));
        for event in &events {
            out.push_str(&format!("  {}", event.event_type.to_string().bright_cyan()));
            if let Some(ref date) = event.date {
                out.push_str(&format!(": {}", date.to_string().yellow()));
            }
            if let Some(ref place_id) = event.place_id {
                if let Ok(place) = db.get_place(place_id) {
                    out.push_str(&format!(" at {}", place.name.green()));
                }
            }
            if let Some(ref notes) = event.notes {
                out.push_str(&format!(" {}", format!("[{}]", notes).bright_black()));
            }
            out.push('\n');
        }
    }

    // Relationships
    if !relationships.is_empty() {
        out.push_str(&format!("\n{}\n", "Relationships:".cyan().bold()));
        for rel in &relationships {
            let other_id = if rel.person1_id == *person_id {
                &rel.person2_id
            } else {
                &rel.person1_id
            };
            if let Ok(other) = db.get_person(other_id) {
                let role = describe_relationship(&rel.rel_type, person_id, &rel.person1_id);
                out.push_str(&format!(
                    "  {} {} {}\n",
                    role.cyan(),
                    other.display_name().bold(),
                    fmt_id(&other.id.to_string())
                ));
            }
        }
    }

    out.push('\n');
    Ok(out)
}

fn describe_relationship(
    rel_type: &RelationshipType,
    subject: &PersonId,
    person1: &PersonId,
) -> &'static str {
    match rel_type {
        RelationshipType::Spouse => "Spouse:    ",
        RelationshipType::Sibling => "Sibling:   ",
        RelationshipType::ParentChild => {
            if subject == person1 {
                "Parent of: "
            } else {
                "Child of:  "
            }
        }
    }
}

// ─── people list report ─────────────────────────────────────────────────────

/// Generate a colored list of all people, showing birth year where known.
pub fn people_list_report(db: &Database) -> KinforgeResult<String> {
    let people = db.list_people()?;
    let mut out = String::new();

    out.push_str(&format!(
        "{}\n\n",
        format!("  {} people in database  ", people.len())
            .bold()
            .bright_cyan()
            .on_black()
    ));

    for person in &people {
        let birth_year = db
            .list_events_for_person(&person.id)
            .ok()
            .and_then(|evts| {
                evts.into_iter()
                    .find(|e| matches!(e.event_type, EventType::Birth))
                    .and_then(|e| e.date)
                    .and_then(|d| year_from_date(&d))
            })
            .map(|y| format!(" b.{}", y))
            .unwrap_or_default();

        out.push_str(&format!(
            "  {} {} {}{}\n",
            fmt_id(&person.id.to_string()),
            person.display_name().bold(),
            format!("({})", person.sex).bright_black(),
            birth_year.yellow()
        ));
    }
    Ok(out)
}

// ─── ancestor report ────────────────────────────────────────────────────────

/// Generate a colored ancestor report with Ahnentafel numbering.
///
/// Ahnentafel: subject = 1, father = 2, mother = 3, pat-grandfather = 4, etc.
pub fn ancestor_report(
    db: &Database,
    person_id: &PersonId,
    generations: u32,
) -> KinforgeResult<String> {
    let mut out = String::new();
    out.push_str(&format!(
        "{}\n\n",
        "  Ancestor Report (Ahnentafel)  "
            .bold()
            .bright_cyan()
            .on_black()
    ));
    collect_ancestors(db, person_id, 1, generations, &mut out)?;
    Ok(out)
}

fn collect_ancestors(
    db: &Database,
    person_id: &PersonId,
    ahnentafel: u64,
    max_gen: u32,
    out: &mut String,
) -> KinforgeResult<()> {
    let person = db.get_person(person_id)?;
    let gen = ahnentafel.ilog2();
    let indent = "  ".repeat(gen as usize);

    out.push_str(&format!(
        "{}[{}] {}\n",
        indent,
        ahnentafel.to_string().yellow(),
        person.display_name().bold()
    ));

    if gen >= max_gen {
        return Ok(());
    }

    let rels = db.list_relationships_for_person(person_id)?;
    let mut parents: Vec<&PersonId> = rels
        .iter()
        .filter(|r| r.rel_type == RelationshipType::ParentChild && r.person2_id == *person_id)
        .map(|r| &r.person1_id)
        .collect();

    parents.sort_by_key(|pid| {
        db.get_person(pid)
            .map(|p| match p.sex {
                Sex::Male => 0u8,
                _ => 1u8,
            })
            .unwrap_or(1)
    });

    for (i, parent_id) in parents.iter().enumerate() {
        let child_ahnentafel = ahnentafel * 2 + i as u64;
        collect_ancestors(db, parent_id, child_ahnentafel, max_gen, out)?;
    }
    Ok(())
}

// ─── descendant report ──────────────────────────────────────────────────────

/// Generate a colored descendant report up to a given number of generations.
pub fn descendant_report(
    db: &Database,
    person_id: &PersonId,
    generations: u32,
) -> KinforgeResult<String> {
    let mut out = String::new();
    out.push_str(&format!(
        "{}\n\n",
        "  Descendant Report  ".bold().bright_cyan().on_black()
    ));
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

    let birth_year = db
        .list_events_for_person(person_id)
        .ok()
        .and_then(|evts| {
            evts.into_iter()
                .find(|e| matches!(e.event_type, EventType::Birth))
                .and_then(|e| e.date)
                .and_then(|d| match d {
                    EventDate::Exact(nd) | EventDate::Approximate(nd) => {
                        Some(nd.format("b.%Y").to_string())
                    }
                    _ => None,
                })
        });

    if let Some(ref year) = birth_year {
        out.push_str(&format!(
            "{}{} {}\n",
            indent,
            person.display_name().bold(),
            format!("({})", year).yellow()
        ));
    } else {
        out.push_str(&format!("{}{}\n", indent, person.display_name().bold()));
    }

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

