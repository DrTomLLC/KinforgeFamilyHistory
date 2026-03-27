use chrono::NaiveDate;
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

    // All names with indexes (so user knows which index to use for update-name / delete-name)
    if !person.names.is_empty() {
        out.push_str(&format!("\n{}\n", "Names:".cyan().bold()));
        for (i, name) in person.names.iter().enumerate() {
            out.push_str(&format!(
                "  {} {} {}\n",
                format!("[{}]", i).yellow(),
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
        RelationshipType::Spouse => "Spouse:      ",
        RelationshipType::Sibling => "Sibling:     ",
        RelationshipType::HalfSibling => "Half-sibling:",
        RelationshipType::ParentChild => {
            if subject == person1 {
                "Parent of:   "
            } else {
                "Child of:    "
            }
        }
        RelationshipType::AdoptiveParent => {
            if subject == person1 {
                "Adopted:     "
            } else {
                "Adoptive par:"
            }
        }
        RelationshipType::Godparent => {
            if subject == person1 {
                "Godparent of:"
            } else {
                "Godchild of: "
            }
        }
        RelationshipType::StepParent => {
            if subject == person1 {
                "Step-parent: "
            } else {
                "Step-child:  "
            }
        }
        RelationshipType::Foster => {
            if subject == person1 {
                "Fostered:    "
            } else {
                "Foster par:  "
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

// ─── family group sheet ─────────────────────────────────────────────────────

/// Generate a Family Group Sheet for a person.
///
/// Shows the person's vital events, their spouse(s), and all children with
/// each child's birth and death dates — the standard genealogical family unit
/// report.
pub fn family_group_sheet(db: &Database, person_id: &PersonId) -> KinforgeResult<String> {
    let person = db.get_person(person_id)?;
    let mut out = String::new();

    out.push_str(&format!(
        "{}\n",
        "  Family Group Sheet  ".bold().bright_cyan().on_black()
    ));
    out.push_str(&format!("{}\n\n", "─".repeat(40).bright_black()));

    // ── Primary person ──────────────────────────────────────────────────
    out.push_str(&format!(
        "{} {}\n",
        "Subject:".cyan().bold(),
        person.display_name().bold()
    ));
    out.push_str(&format!(
        "{}  {}\n",
        "ID:     ".cyan(),
        person.id.to_string().bright_black()
    ));
    out.push_str(&format!("{}  {}\n", "Sex:    ".cyan(), fmt_sex(&person.sex)));
    append_vital_events(db, person_id, &mut out)?;

    let rels = db.list_relationships_for_person(person_id)?;

    // ── Spouses ─────────────────────────────────────────────────────────
    let spouses: Vec<&PersonId> = rels
        .iter()
        .filter(|r| r.rel_type == RelationshipType::Spouse)
        .map(|r| {
            if r.person1_id == *person_id {
                &r.person2_id
            } else {
                &r.person1_id
            }
        })
        .collect();

    if !spouses.is_empty() {
        out.push_str(&format!(
            "\n{}\n",
            format!("  {} Spouse(s)  ", spouses.len())
                .bold()
                .bright_cyan()
                .on_black()
        ));
        for spouse_id in &spouses {
            if let Ok(spouse) = db.get_person(spouse_id) {
                out.push_str(&format!(
                    "  {} {} {}\n",
                    fmt_sex(&spouse.sex),
                    spouse.display_name().bold(),
                    spouse.id.to_string().bright_black()
                ));
                append_vital_events(db, spouse_id, &mut out)?;
            }
        }
    }

    // ── Children ─────────────────────────────────────────────────────────
    let children: Vec<&PersonId> = rels
        .iter()
        .filter(|r| r.rel_type == RelationshipType::ParentChild && r.person1_id == *person_id)
        .map(|r| &r.person2_id)
        .collect();

    if !children.is_empty() {
        out.push_str(&format!(
            "\n{}\n",
            format!("  {} Child(ren)  ", children.len())
                .bold()
                .bright_cyan()
                .on_black()
        ));
        for (i, child_id) in children.iter().enumerate() {
            if let Ok(child) = db.get_person(child_id) {
                let events = db.list_events_for_person(child_id)?;
                let birth = events
                    .iter()
                    .find(|e| matches!(e.event_type, EventType::Birth))
                    .and_then(|e| e.date.as_ref())
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let death = events
                    .iter()
                    .find(|e| matches!(e.event_type, EventType::Death))
                    .and_then(|e| e.date.as_ref())
                    .map(|d| format!(" \u{2013} d.{}", d))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "  {}. {} {} {} {}{}\n",
                    (i + 1).to_string().yellow(),
                    fmt_sex(&child.sex),
                    child.display_name().bold(),
                    child.id.to_string().bright_black(),
                    format!("b.{}", birth).yellow(),
                    death.yellow()
                ));
            }
        }
    }

    out.push('\n');
    Ok(out)
}

// ─── timeline report ─────────────────────────────────────────────────────────

/// Generate a chronological timeline of all recorded events for a person.
pub fn timeline_report(db: &Database, person_id: &PersonId) -> KinforgeResult<String> {
    let person = db.get_person(person_id)?;
    let mut events = db.list_events_for_person(person_id)?;
    let mut out = String::new();

    let header = format!("  Timeline: {}  ", person.display_name());
    out.push_str(&format!("{}\n", header.bold().bright_cyan().on_black()));
    out.push_str(&format!("{}\n\n", "─".repeat(header.len()).bright_black()));

    if events.is_empty() {
        out.push_str(&format!("{}\n", "No events recorded.".bright_black()));
        return Ok(out);
    }

    // Sort: events with a date first (chronologically), undated last.
    events.sort_by(|a, b| {
        let key = |e: &Event| -> Option<NaiveDate> {
            e.date.as_ref().and_then(|d| match d {
                EventDate::Exact(nd)
                | EventDate::Approximate(nd)
                | EventDate::Before(nd)
                | EventDate::After(nd)
                | EventDate::Between(nd, _) => Some(*nd),
                EventDate::Unknown => None,
            })
        };
        match (key(a), key(b)) {
            (Some(da), Some(db)) => da.cmp(&db),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    for e in &events {
        let date_str = e
            .date
            .as_ref()
            .map(|d| d.to_string().yellow().to_string())
            .unwrap_or_else(|| "undated".bright_black().to_string());
        let place_str = e
            .place_id
            .as_ref()
            .and_then(|pid| db.get_place(pid).ok())
            .map(|pl| format!(" @ {}", pl.name.green()))
            .unwrap_or_default();
        let notes_str = e
            .notes
            .as_deref()
            .map(|n| format!(" {}", format!("[{}]", n).bright_black()))
            .unwrap_or_default();
        out.push_str(&format!(
            "  {:<12} {}{}{}\n",
            e.event_type.to_string().bright_cyan(),
            date_str,
            place_str,
            notes_str
        ));
    }

    out.push('\n');
    Ok(out)
}

/// Append birth, death, marriage, baptism, burial events indented under the entry.
fn append_vital_events(
    db: &Database,
    person_id: &PersonId,
    out: &mut String,
) -> KinforgeResult<()> {
    let events = db.list_events_for_person(person_id)?;
    let vital_types = [
        EventType::Birth,
        EventType::Baptism,
        EventType::Marriage,
        EventType::Death,
        EventType::Burial,
    ];
    for vt in &vital_types {
        if let Some(e) = events
            .iter()
            .find(|e| std::mem::discriminant(&e.event_type) == std::mem::discriminant(vt))
        {
            let date_str = e
                .date
                .as_ref()
                .map(|d| d.to_string().yellow().to_string())
                .unwrap_or_else(|| "?".bright_black().to_string());
            let place_str = e
                .place_id
                .as_ref()
                .and_then(|pid| db.get_place(pid).ok())
                .map(|pl| format!(" @ {}", pl.name.green()))
                .unwrap_or_default();
            out.push_str(&format!(
                "         {:<10} {}{}\n",
                e.event_type.to_string().bright_cyan(),
                date_str,
                place_str
            ));
        }
    }
    Ok(())
}
// ─── sources report ──────────────────────────────────────────────────────────

/// Generate a colored list of all sources with per-source citation counts.
pub fn sources_report(db: &Database) -> KinforgeResult<String> {
    let sources = db.list_sources()?;
    let mut out = String::new();

    out.push_str(&format!(
        "{}\n\n",
        format!("  {} source(s) in database  ", sources.len())
            .bold()
            .bright_cyan()
            .on_black()
    ));

    if sources.is_empty() {
        out.push_str(&format!("{}\n", "No sources recorded.".bright_black()));
        return Ok(out);
    }

    for source in &sources {
        let citation_count = db.list_citations_for_source(&source.id)
            .map(|c| c.len())
            .unwrap_or(0);

        let year_str = source
            .year
            .map(|y| format!(" {}", format!("({})", y).yellow()))
            .unwrap_or_default();
        let author_str = source
            .author
            .as_deref()
            .map(|a| format!(" {}", format!("— {}", a).bright_black()))
            .unwrap_or_default();
        let cit_str = if citation_count == 0 {
            format!(" {}", "[no citations]".bright_red())
        } else {
            format!(" {}", format!("[{} citation(s)]", citation_count).bright_black())
        };

        out.push_str(&format!(
            "  {} {}{}{}{}\n",
            fmt_id(&source.id.to_string()),
            source.title.bold(),
            year_str,
            author_str,
            cit_str
        ));
    }
    Ok(out)
}

// ─── narrative report ────────────────────────────────────────────────────────

/// Generate a prose-style narrative biography for a person.
pub fn narrative_report(db: &Database, person_id: &PersonId) -> KinforgeResult<String> {
    use kinforge_core::models::EventType;

    let person = db.get_person(person_id)?;
    let name = person.display_name();
    let events = db.list_events_for_person(person_id)?;
    let relationships = db.list_relationships_for_person(person_id)?;

    let mut out = String::new();
    out.push_str(&format!(
        "\n{}\n{}\n\n",
        format!("  Narrative Biography: {}  ", name)
            .bold()
            .bright_cyan()
            .on_black(),
        "─".repeat(60).bright_black()
    ));

    // Helper: get event of given type
    let get_event = |etype: EventType| {
        events.iter().find(|e| e.event_type == etype).cloned()
    };

    // ── Birth sentence ────────────────────────────────────────────────────
    let birth = get_event(EventType::Birth);
    let baptism = get_event(EventType::Baptism);

    let pronoun = match person.sex {
        kinforge_core::models::Sex::Male => "He",
        kinforge_core::models::Sex::Female => "She",
        kinforge_core::models::Sex::Unknown => "They",
    };

    if let Some(ref b) = birth {
        let date_str = b.date.as_ref().map(format_date_prose).unwrap_or_default();
        let place_str = if let Some(ref pid) = b.place_id {
            db.get_place(pid)
                .ok()
                .map(|p| format!(" in {}", p.name))
                .unwrap_or_default()
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{} was born{}{}.  ",
            name.bold(),
            if date_str.is_empty() { String::new() } else { format!(" {}", date_str) },
            place_str
        ));
    } else {
        out.push_str(&format!("{} was born on an unknown date.  ", name.bold()));
    }

    if let Some(ref bap) = baptism {
        let date_str = bap.date.as_ref().map(format_date_prose).unwrap_or_default();
        let place_str = if let Some(ref pid) = bap.place_id {
            db.get_place(pid)
                .ok()
                .map(|p| format!(" at {}", p.name))
                .unwrap_or_default()
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{} was baptised{}{}.",
            pronoun,
            if date_str.is_empty() { String::new() } else { format!(" {}", date_str) },
            place_str
        ));
    }
    out.push('\n');

    // ── Parents ───────────────────────────────────────────────────────────
    let parent_rels: Vec<_> = relationships
        .iter()
        .filter(|r| {
            (r.rel_type == kinforge_core::models::RelationshipType::ParentChild
                || r.rel_type == kinforge_core::models::RelationshipType::AdoptiveParent)
                && r.person2_id == *person_id
        })
        .collect();

    if !parent_rels.is_empty() {
        let parent_names: Vec<String> = parent_rels
            .iter()
            .filter_map(|r| {
                db.get_person(&r.person1_id)
                    .ok()
                    .map(|p| {
                        if r.rel_type == kinforge_core::models::RelationshipType::AdoptiveParent {
                            format!("{} (adoptive)", p.display_name())
                        } else {
                            p.display_name()
                        }
                    })
            })
            .collect();
        out.push_str(&format!(
            "{} was the child of {}.\n",
            pronoun,
            join_names(&parent_names)
        ));
    }

    // ── Marriages ─────────────────────────────────────────────────────────
    let spouse_rels: Vec<_> = relationships
        .iter()
        .filter(|r| r.rel_type == kinforge_core::models::RelationshipType::Spouse)
        .collect();

    for rel in &spouse_rels {
        let spouse_id = if rel.person1_id == *person_id {
            &rel.person2_id
        } else {
            &rel.person1_id
        };
        if let Ok(spouse) = db.get_person(spouse_id) {
            // Look for marriage event near this — for simplicity, emit a generic sentence
            out.push_str(&format!(
                "{} married {}.\n",
                pronoun,
                spouse.display_name().bold()
            ));
        }
    }

    // ── Children ──────────────────────────────────────────────────────────
    let child_rels: Vec<_> = relationships
        .iter()
        .filter(|r| {
            r.rel_type == kinforge_core::models::RelationshipType::ParentChild
                && r.person1_id == *person_id
        })
        .collect();

    if !child_rels.is_empty() {
        let child_names: Vec<String> = child_rels
            .iter()
            .filter_map(|r| db.get_person(&r.person2_id).ok().map(|p| p.display_name()))
            .collect();
        let count = child_names.len();
        out.push_str(&format!(
            "{} had {} {}: {}.\n",
            pronoun,
            count,
            if count == 1 { "child" } else { "children" },
            join_names(&child_names)
        ));
    }

    // ── Other events (excluding Birth/Baptism) ────────────────────────────
    let mut other_events: Vec<_> = events
        .iter()
        .filter(|e| {
            !matches!(
                e.event_type,
                EventType::Birth | EventType::Baptism
            )
        })
        .collect();
    other_events.sort_by(|a, b| {
        let key = |e: &&kinforge_core::models::Event| -> i32 {
            e.date.as_ref().and_then(|d| {
                use kinforge_core::models::EventDate;
                use chrono::Datelike;
                match d {
                    EventDate::Exact(nd) | EventDate::Approximate(nd) => Some(nd.year()),
                    _ => None,
                }
            }).unwrap_or(i32::MAX)
        };
        key(a).cmp(&key(b))
    });

    for event in &other_events {
        let date_str = event.date.as_ref().map(format_date_prose).unwrap_or_default();
        let place_str = if let Some(ref pid) = event.place_id {
            db.get_place(pid)
                .ok()
                .map(|p| format!(" in {}", p.name))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let event_verb = match event.event_type {
            EventType::Death => format!("{} died{}{}", pronoun, if date_str.is_empty() { String::new() } else { format!(" {}", date_str) }, place_str),
            EventType::Burial => format!("{} was buried{}{}", pronoun, if date_str.is_empty() { String::new() } else { format!(" {}", date_str) }, place_str),
            EventType::Marriage => format!("{} married{}{}", pronoun, if date_str.is_empty() { String::new() } else { format!(" {}", date_str) }, place_str),
            EventType::Divorce => format!("{} was divorced{}", pronoun, if date_str.is_empty() { String::new() } else { format!(" {}", date_str) }),
            EventType::Emigration => format!("{} emigrated{}{}", pronoun, if date_str.is_empty() { String::new() } else { format!(" {}", date_str) }, place_str),
            EventType::Immigration => format!("{} immigrated{}{}", pronoun, if date_str.is_empty() { String::new() } else { format!(" {}", date_str) }, place_str),
            EventType::Census => format!("{} appeared in a census{}{}", pronoun, if date_str.is_empty() { String::new() } else { format!(" {}", date_str) }, place_str),
            EventType::Occupation => {
                let desc = event.notes.as_deref().unwrap_or("unknown occupation");
                format!("{} worked as {}", pronoun, desc)
            }
            EventType::Residence => format!("{} resided{}{}", pronoun, if date_str.is_empty() { String::new() } else { format!(" {}", date_str) }, place_str),
            ref other => format!("{} [{}]{}{}", pronoun, format!("{:?}", other).to_lowercase(), if date_str.is_empty() { String::new() } else { format!(" {}", date_str) }, place_str),
        };
        let notes_str = event
            .notes
            .as_deref()
            .filter(|_| event.event_type != EventType::Occupation)
            .map(|n| format!(" ({})", n))
            .unwrap_or_default();
        out.push_str(&format!("{}{}.\n", event_verb, notes_str));
    }

    // ── Notes ─────────────────────────────────────────────────────────────
    if let Some(ref notes) = person.notes {
        out.push('\n');
        out.push_str(&format!("{} {}\n", "Notes:".cyan(), notes));
    }

    out.push_str(&format!("\n{}\n", "─".repeat(60).bright_black()));
    Ok(out)
}

fn format_date_prose(date: &EventDate) -> String {
    match date {
        EventDate::Exact(nd) => format!("on {}", nd.format("%d %B %Y")),
        EventDate::Approximate(nd) => format!("around {}", nd.format("%Y")),
        EventDate::Before(nd) => format!("before {}", nd.format("%Y")),
        EventDate::After(nd) => format!("after {}", nd.format("%Y")),
        EventDate::Between(nd1, nd2) => {
            format!("between {} and {}", nd1.format("%Y"), nd2.format("%Y"))
        }
        EventDate::Unknown => String::new(),
    }
}

fn join_names(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        2 => format!("{} and {}", names[0], names[1]),
        _ => {
            let (last, rest) = names.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}
