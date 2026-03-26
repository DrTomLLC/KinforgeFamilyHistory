use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;

/// Generate a plain-text individual summary report.
pub fn individual_report(db: &Database, person_id: &PersonId) -> KinforgeResult<String> {
    let person = db.get_person(person_id)?;
    let events = db.list_events_for_person(person_id)?;
    let relationships = db.list_relationships_for_person(person_id)?;

    let mut out = String::new();

    out.push_str(&format!("=== {} ===\n", person.display_name()));
    out.push_str(&format!("ID:  {}\n", person.id));
    out.push_str(&format!("Sex: {}\n", person.sex));

    // Birth / Death summary line
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
    out.push_str(&format!("Life: {} — {}\n", born_str, died_str));

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

/// Generate a plain-text list of all people, showing birth year where known.
pub fn people_list_report(db: &Database) -> KinforgeResult<String> {
    let people = db.list_people()?;
    let mut out = String::new();
    out.push_str(&format!("Total people: {}\n\n", people.len()));
    for person in &people {
        // Try to find a birth year
        let birth_year = db
            .list_events_for_person(&person.id)
            .ok()
            .and_then(|evts| {
                evts.into_iter()
                    .find(|e| matches!(e.event_type, EventType::Birth))
                    .and_then(|e| e.date)
                    .and_then(|d| match d {
                        EventDate::Exact(nd)
                        | EventDate::Approximate(nd)
                        | EventDate::Before(nd)
                        | EventDate::After(nd) => Some(nd.format("%Y").to_string()),
                        EventDate::Between(nd, _) => Some(nd.format("%Y").to_string()),
                        EventDate::Unknown => None,
                    })
            })
            .map(|y| format!(" b.{}", y))
            .unwrap_or_default();

        out.push_str(&format!(
            "  [{}] {} ({}){}  \n",
            person.id,
            person.display_name(),
            person.sex,
            birth_year
        ));
    }
    Ok(out)
}

/// Generate a plain-text ancestor report with Ahnentafel numbering.
///
/// Ahnentafel: subject = 1, father = 2, mother = 3, pat-grandfather = 4, etc.
pub fn ancestor_report(
    db: &Database,
    person_id: &PersonId,
    generations: u32,
) -> KinforgeResult<String> {
    let mut out = String::new();
    out.push_str("=== Ancestor Report (Ahnentafel) ===\n\n");
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
    let gen = ahnentafel.ilog2(); // generation 0 for subject, 1 for parents, etc.
    let indent = "  ".repeat(gen as usize);
    out.push_str(&format!(
        "{}[{}] {}\n",
        indent,
        ahnentafel,
        person.display_name()
    ));

    if gen >= max_gen {
        return Ok(());
    }

    // Find parents: ParentChild where person is person2 (child)
    let rels = db.list_relationships_for_person(person_id)?;
    let mut parents: Vec<&PersonId> = rels
        .iter()
        .filter(|r| r.rel_type == RelationshipType::ParentChild && r.person2_id == *person_id)
        .map(|r| &r.person1_id)
        .collect();

    // Assign father (male) to even slot, mother (female/unknown) to odd slot
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

    // Show birth year inline if available
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
        })
        .map(|y| format!(" ({})", y))
        .unwrap_or_default();

    out.push_str(&format!(
        "{}{}{}\n",
        indent,
        person.display_name(),
        birth_year
    ));

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

// ── Ahnentafel Table ─────────────────────────────────────────────────────────

/// Generate a printable Ahnentafel (ancestor) table.
///
/// Each ancestor gets a numbered row: 1 = subject, 2 = father, 3 = mother,
/// 4 = pat. grandfather, etc.  Birth and death dates are shown inline.
pub fn ahnentafel_table(
    db: &Database,
    person_id: &PersonId,
    generations: u32,
) -> KinforgeResult<String> {
    let mut rows: Vec<(u64, String)> = Vec::new();
    collect_ahnentafel_rows(db, person_id, 1, generations, &mut rows)?;
    rows.sort_by_key(|(n, _)| *n);

    let mut out = String::new();
    out.push_str("╔══════════════════════════════════════════════════════════════╗\n");
    out.push_str("║                  AHNENTAFEL ANCESTOR TABLE                  ║\n");
    out.push_str("╚══════════════════════════════════════════════════════════════╝\n\n");

    let header = format!("{:<6} {:<30} {:<12} {:<12}", "#", "Name", "Born", "Died");
    out.push_str(&header);
    out.push('\n');
    out.push_str(&"─".repeat(62));
    out.push('\n');

    for (_, line) in &rows {
        out.push_str(line);
        out.push('\n');
    }

    out.push('\n');
    out.push_str(
        "Generation key: 1=subject  2-3=parents  4-7=grandparents  8-15=great-grandparents\n",
    );
    Ok(out)
}

fn collect_ahnentafel_rows(
    db: &Database,
    person_id: &PersonId,
    num: u64,
    max_gen: u32,
    rows: &mut Vec<(u64, String)>,
) -> KinforgeResult<()> {
    let person = db.get_person(person_id)?;
    let events = db.list_events_for_person(person_id).unwrap_or_default();

    let born = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Birth))
        .and_then(|e| e.date.as_ref())
        .map(date_year_str)
        .unwrap_or_else(|| "?".to_string());

    let died = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Death))
        .and_then(|e| e.date.as_ref())
        .map(date_year_str)
        .unwrap_or_default();

    let line = format!(
        "{:<6} {:<30} {:<12} {:<12}",
        num,
        truncate(&person.display_name(), 30),
        born,
        died,
    );
    rows.push((num, line));

    let gen = num.ilog2();
    if gen >= max_gen {
        return Ok(());
    }

    let rels = db.list_relationships_for_person(person_id)?;
    let mut parents: Vec<PersonId> = rels
        .iter()
        .filter(|r| r.rel_type == RelationshipType::ParentChild && r.person2_id == *person_id)
        .map(|r| r.person1_id.clone())
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
        collect_ahnentafel_rows(db, parent_id, num * 2 + i as u64, max_gen, rows)?;
    }
    Ok(())
}

fn date_year_str(d: &EventDate) -> String {
    match d {
        EventDate::Exact(nd) | EventDate::Approximate(nd) => nd.format("%Y").to_string(),
        EventDate::Before(nd) => format!("<{}", nd.format("%Y")),
        EventDate::After(nd) => format!(">{}", nd.format("%Y")),
        EventDate::Between(d1, d2) => {
            format!("{}-{}", d1.format("%Y"), d2.format("%Y"))
        }
        EventDate::Unknown => "?".to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

// ── Family Group Sheet ────────────────────────────────────────────────────────

/// Generate a family group sheet for a person: shows the person, their spouses,
/// and children grouped by family unit.
pub fn family_group_sheet(db: &Database, person_id: &PersonId) -> KinforgeResult<String> {
    let person = db.get_person(person_id)?;
    let events = db.list_events_for_person(person_id).unwrap_or_default();
    let rels = db.list_relationships_for_person(person_id)?;

    let mut out = String::new();
    out.push_str("╔══════════════════════════════════════════════════════════════╗\n");
    out.push_str("║                     FAMILY GROUP SHEET                      ║\n");
    out.push_str("╚══════════════════════════════════════════════════════════════╝\n\n");

    // ── Subject ────────────────────────────────────────────────────────────────
    out.push_str(&format!(
        "SUBJECT: {} ({})\n",
        person.display_name(),
        person.sex
    ));
    write_vital_events(&mut out, db, &events, "  ");
    if let Some(ref n) = person.notes {
        out.push_str(&format!("  Notes: {}\n", n));
    }
    out.push('\n');

    // ── Parents ────────────────────────────────────────────────────────────────
    let parents: Vec<_> = rels
        .iter()
        .filter(|r| r.rel_type == RelationshipType::ParentChild && r.person2_id == *person_id)
        .collect();
    if !parents.is_empty() {
        out.push_str("PARENTS:\n");
        for r in &parents {
            if let Ok(parent) = db.get_person(&r.person1_id) {
                let parent_events = db.list_events_for_person(&r.person1_id).unwrap_or_default();
                out.push_str(&format!("  {} ({})\n", parent.display_name(), parent.sex));
                write_vital_events(&mut out, db, &parent_events, "    ");
            }
        }
        out.push('\n');
    }

    // ── Spouses + children ────────────────────────────────────────────────────
    let spouses: Vec<_> = rels
        .iter()
        .filter(|r| r.rel_type == RelationshipType::Spouse)
        .collect();

    if spouses.is_empty() {
        // Show children with unknown other parent
        let children = children_of(db, person_id, &rels);
        if !children.is_empty() {
            out.push_str("CHILDREN:\n");
            write_children(&mut out, db, &children);
        }
    } else {
        for (i, spouse_rel) in spouses.iter().enumerate() {
            let spouse_id = if spouse_rel.person1_id == *person_id {
                &spouse_rel.person2_id
            } else {
                &spouse_rel.person1_id
            };

            out.push_str(&format!("FAMILY {}:\n", i + 1));
            if let Ok(spouse) = db.get_person(spouse_id) {
                let spouse_events = db.list_events_for_person(spouse_id).unwrap_or_default();
                out.push_str(&format!(
                    "  SPOUSE: {} ({})\n",
                    spouse.display_name(),
                    spouse.sex
                ));
                write_vital_events(&mut out, db, &spouse_events, "    ");
                if let Some(ref n) = spouse_rel.notes {
                    out.push_str(&format!("    Union notes: {}\n", n));
                }
            }

            let children = children_of(db, person_id, &rels);
            if !children.is_empty() {
                out.push_str("  CHILDREN:\n");
                write_children(&mut out, db, &children);
            }
            out.push('\n');
        }
    }

    Ok(out)
}

fn write_vital_events(out: &mut String, db: &Database, events: &[Event], indent: &str) {
    for e in events {
        match e.event_type {
            EventType::Birth | EventType::Death | EventType::Marriage | EventType::Burial => {
                let date_str = e.date.as_ref().map(|d| d.to_string()).unwrap_or_default();
                let place_str = e
                    .place_id
                    .as_ref()
                    .and_then(|pid| db.get_place(pid).ok())
                    .map(|pl| format!(", {}", pl.name))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "{}{}: {}{}\n",
                    indent, e.event_type, date_str, place_str
                ));
            }
            _ => {}
        }
    }
}

fn children_of<'a>(
    db: &Database,
    person_id: &PersonId,
    rels: &'a [Relationship],
) -> Vec<&'a PersonId> {
    let _ = db; // may be used for ordering in future
    rels.iter()
        .filter(|r| r.rel_type == RelationshipType::ParentChild && r.person1_id == *person_id)
        .map(|r| &r.person2_id)
        .collect()
}

fn write_children(out: &mut String, db: &Database, child_ids: &[&PersonId]) {
    for child_id in child_ids {
        if let Ok(child) = db.get_person(child_id) {
            let child_events = db.list_events_for_person(child_id).unwrap_or_default();
            let born = child_events
                .iter()
                .find(|e| matches!(e.event_type, EventType::Birth))
                .and_then(|e| e.date.as_ref())
                .map(|d| format!(" b.{}", date_year_str(d)))
                .unwrap_or_default();
            out.push_str(&format!(
                "    {} ({}){}  \n",
                child.display_name(),
                child.sex,
                born
            ));
        }
    }
}
