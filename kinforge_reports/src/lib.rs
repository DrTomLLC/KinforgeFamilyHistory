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

    // Research Tasks linked to this person
    let tasks = db.list_tasks_for_person(person_id).unwrap_or_default();
    if !tasks.is_empty() {
        out.push_str(&format!("\n{}\n", "Research Tasks:".cyan().bold()));
        for task in &tasks {
            let status_marker = match task.status {
                TaskStatus::Pending => "[ ]".bright_black(),
                TaskStatus::InProgress => "[~]".yellow(),
                TaskStatus::Done => "[✓]".green(),
            };
            out.push_str(&format!(
                "  {} {}\n",
                status_marker,
                task.description.bold()
            ));
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

// ─── HTML export ─────────────────────────────────────────────────────────────

const HTML_CSS: &str = r#"
  body{font-family:Georgia,'Times New Roman',serif;max-width:1100px;margin:0 auto;padding:24px 16px;background:#f9f7f4;color:#222;line-height:1.5}
  h1{font-size:2em;border-bottom:3px solid #8b6a3e;padding-bottom:.3em;color:#4a3520;margin-bottom:.3em}
  h2{font-size:1.3em;color:#6b4c2a;margin:2em 0 .4em;border-bottom:1px solid #d4c4a8;padding-bottom:.2em}
  h3{font-size:1.1em;color:#4a3520;margin:0 0 6px}
  .subtitle{color:#888;font-size:.9em;margin-bottom:2em}
  #toc{background:#fff8f0;border:1px solid #d4c4a8;border-radius:4px;padding:16px;margin-bottom:2em}
  #toc h2{margin:0 0 .5em;border:none}
  #toc table{border-collapse:collapse;width:100%}
  #toc th{background:#8b6a3e;color:#fff;text-align:left;padding:5px 10px;font-size:.85em}
  #toc td{padding:4px 10px;font-size:.87em;border-bottom:1px solid #ede4d4}
  #toc tr:hover td{background:#fff3e0}
  .person-card{background:#fff;border:1px solid #d4c4a8;border-radius:6px;padding:18px 20px;margin:18px 0;box-shadow:0 1px 3px rgba(0,0,0,.06)}
  .person-id{font-family:monospace;font-size:.72em;color:#bbb;margin-left:8px}
  .life-dates{color:#777;font-size:.9em;margin-bottom:8px}
  .sex-male{color:#2a5a8c}.sex-female{color:#8c2a5a}.sex-unknown{color:#777}
  table.events{border-collapse:collapse;width:100%;margin:8px 0 12px;font-size:.87em}
  table.events th{background:#f0e8d8;text-align:left;padding:4px 8px;color:#5a3a10}
  table.events td{padding:4px 8px;border-bottom:1px solid #ede4d4;vertical-align:top}
  .rel-section{margin:8px 0 0;font-size:.88em}
  .rel-section strong{color:#6b4c2a}
  .rel-item{display:inline-block;margin:2px 4px 2px 0}
  a{color:#8b4513;text-decoration:none}
  a:hover{text-decoration:underline;color:#c06a20}
  .badge{display:inline-block;padding:1px 7px;border-radius:10px;font-size:.75em;font-family:monospace;background:#ede4d4;color:#6b4c2a;border:1px solid #d4c4a8}
  .notes{font-style:italic;color:#888;font-size:.85em;margin-top:6px}
  .no-events{color:#bbb;font-size:.85em;font-style:italic}
  footer{margin-top:3em;padding-top:1em;border-top:1px solid #d4c4a8;font-size:.8em;color:#aaa;text-align:center}
"#;

/// Export all data as a self-contained single-file HTML document.
pub fn html_export(db: &Database) -> KinforgeResult<String> {
    use kinforge_core::models::{EventType, Sex};
    use chrono::Local;

    let people = db.list_people()?;
    let today = Local::now().format("%d %B %Y").to_string();

    let mut out = String::with_capacity(65536);

    // ── Head ──────────────────────────────────────────────────────────────
    out.push_str(&format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>Kinforge Family History</title>
<style>{}</style>
</head>
<body>
<h1>Family History Report</h1>
<p class="subtitle">Generated by Kinforge &bull; {} &bull; {} people</p>
"#,
        HTML_CSS,
        html_escape(&today),
        people.len()
    ));

    // ── Index table ───────────────────────────────────────────────────────
    out.push_str(r#"<div id="toc"><h2>Index of People</h2>
<table>
<tr><th>Name</th><th>Sex</th><th>Born</th><th>Died</th></tr>
"#);

    for p in &people {
        let name = p.display_name();
        let anchor = format!("p-{}", p.id.as_str().replace('-', ""));
        let sex_class = match p.sex {
            Sex::Male => "sex-male",
            Sex::Female => "sex-female",
            Sex::Unknown => "sex-unknown",
        };
        let sex_label = match p.sex {
            Sex::Male => "M",
            Sex::Female => "F",
            Sex::Unknown => "?",
        };
        let events = db.list_events_for_person(&p.id).unwrap_or_default();
        let birth_yr = events
            .iter()
            .find(|e| e.event_type == EventType::Birth)
            .and_then(|e| e.date.as_ref())
            .and_then(|d| year_from_date(d))
            .unwrap_or_default();
        let death_yr = events
            .iter()
            .find(|e| e.event_type == EventType::Death)
            .and_then(|e| e.date.as_ref())
            .and_then(|d| year_from_date(d))
            .unwrap_or_default();

        out.push_str(&format!(
            "<tr><td><a href=\"#{anchor}\">{name}</a></td>\
             <td class=\"{sex_class}\">{sex_label}</td>\
             <td>{birth_yr}</td><td>{death_yr}</td></tr>\n",
            anchor = anchor,
            name = html_escape(&name),
            sex_class = sex_class,
            sex_label = sex_label,
            birth_yr = html_escape(&birth_yr),
            death_yr = html_escape(&death_yr),
        ));
    }
    out.push_str("</table></div>\n");

    // ── Per-person cards ──────────────────────────────────────────────────
    out.push_str("<h2>People</h2>\n");

    for p in &people {
        let name = p.display_name();
        let anchor = format!("p-{}", p.id.as_str().replace('-', ""));
        let events = db.list_events_for_person(&p.id).unwrap_or_default();
        let relationships = db.list_relationships_for_person(&p.id).unwrap_or_default();

        let birth_yr = events.iter().find(|e| e.event_type == EventType::Birth)
            .and_then(|e| e.date.as_ref()).and_then(|d| year_from_date(d));
        let death_yr = events.iter().find(|e| e.event_type == EventType::Death)
            .and_then(|e| e.date.as_ref()).and_then(|d| year_from_date(d));

        let life_dates = match (birth_yr, death_yr) {
            (Some(b), Some(d)) => format!("{} – {}", b, d),
            (Some(b), None) => format!("b. {}", b),
            (None, Some(d)) => format!("d. {}", d),
            (None, None) => String::new(),
        };

        let sex_class = match p.sex {
            Sex::Male => "sex-male",
            Sex::Female => "sex-female",
            Sex::Unknown => "sex-unknown",
        };

        out.push_str(&format!(
            "<div class=\"person-card\" id=\"{anchor}\">\
             <h3><span class=\"{sex_class}\">{name}</span>\
             <span class=\"person-id\">{short_id}</span></h3>\n",
            anchor = anchor,
            sex_class = sex_class,
            name = html_escape(&name),
            short_id = &p.id.as_str()[..8],
        ));

        if !life_dates.is_empty() {
            out.push_str(&format!(
                "<div class=\"life-dates\">{}</div>\n",
                html_escape(&life_dates)
            ));
        }

        // Events table
        if events.is_empty() {
            out.push_str("<p class=\"no-events\">No events recorded.</p>\n");
        } else {
            out.push_str("<table class=\"events\"><tr><th>Event</th><th>Date</th><th>Place</th><th>Notes</th></tr>\n");
            for e in &events {
                let date_str = e.date.as_ref()
                    .map(|d| {
                        use kinforge_core::models::EventDate;
                        match d {
                            EventDate::Exact(nd) => nd.format("%d %b %Y").to_string(),
                            EventDate::Approximate(nd) => format!("c. {}", nd.format("%Y")),
                            EventDate::Before(nd) => format!("bef. {}", nd.format("%Y")),
                            EventDate::After(nd) => format!("aft. {}", nd.format("%Y")),
                            EventDate::Between(n1, n2) => format!("{}/{}",n1.format("%Y"),n2.format("%Y")),
                            EventDate::Unknown => String::new(),
                        }
                    })
                    .unwrap_or_default();
                let place_str = e.place_id.as_ref()
                    .and_then(|pid| db.get_place(pid).ok())
                    .map(|pl| pl.name.clone())
                    .unwrap_or_default();
                out.push_str(&format!(
                    "<tr><td>{etype}</td><td>{date}</td><td>{place}</td><td>{notes}</td></tr>\n",
                    etype = html_escape(&e.event_type.to_string()),
                    date = html_escape(&date_str),
                    place = html_escape(&place_str),
                    notes = html_escape(e.notes.as_deref().unwrap_or("")),
                ));
            }
            out.push_str("</table>\n");
        }

        // Relationships
        if !relationships.is_empty() {
            out.push_str("<div class=\"rel-section\"><strong>Relationships:</strong> ");
            for rel in &relationships {
                let other_id = if rel.person1_id == p.id { &rel.person2_id } else { &rel.person1_id };
                let other_anchor = format!("p-{}", other_id.as_str().replace('-', ""));
                let other_name = db.get_person(other_id)
                    .map(|op| op.display_name())
                    .unwrap_or_else(|_| other_id.to_string());
                let rel_label = rel.rel_type.to_string();
                out.push_str(&format!(
                    "<span class=\"rel-item\"><span class=\"badge\">{rel}</span> \
                     <a href=\"#{anchor}\">{name}</a></span>",
                    rel = html_escape(&rel_label),
                    anchor = other_anchor,
                    name = html_escape(&other_name),
                ));
            }
            out.push_str("</div>\n");
        }

        // Person notes
        if let Some(ref notes) = p.notes {
            out.push_str(&format!(
                "<div class=\"notes\">{}</div>\n",
                html_escape(notes)
            ));
        }

        out.push_str("</div>\n");
    }

    // ── Footer ────────────────────────────────────────────────────────────
    out.push_str(&format!(
        "<footer>Kinforge Family History &bull; {} people &bull; Generated {}</footer>\n\
         </body>\n</html>\n",
        people.len(),
        html_escape(&today)
    ));

    Ok(out)
}

// ─── places report ──────────────────────────────────────────────────────────

/// Generate a colored list of all places, sorted by number of linked events (descending).
pub fn places_report(db: &Database) -> KinforgeResult<String> {
    use std::collections::HashMap;

    let places = db.list_places()?;
    let all_events = db.list_all_events()?;

    // Count events per place
    let mut event_counts: HashMap<String, usize> = HashMap::new();
    for ev in &all_events {
        if let Some(ref pid) = ev.place_id {
            *event_counts.entry(pid.to_string()).or_insert(0) += 1;
        }
    }

    let mut place_rows: Vec<_> = places
        .iter()
        .map(|p| {
            let count = event_counts.get(&p.id.to_string()).copied().unwrap_or(0);
            (p, count)
        })
        .collect();
    place_rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.name.cmp(&b.0.name)));

    let mut out = String::new();
    out.push_str(&format!(
        "{}\n\n",
        format!("  {} place{} in database  ", places.len(), if places.len() == 1 { "" } else { "s" })
            .bold()
            .bright_cyan()
            .on_black()
    ));

    if place_rows.is_empty() {
        out.push_str(&format!("{}\n", "  (no places recorded)".bright_black()));
        return Ok(out);
    }

    for (place, count) in &place_rows {
        let coord_str = match (place.latitude, place.longitude) {
            (Some(lat), Some(lon)) => format!(" {}", format!("({:.4}°, {:.4}°)", lat, lon).bright_black()),
            _ => String::new(),
        };
        let parent_str = if let Some(ref par_id) = place.parent_id {
            db.get_place(par_id)
                .map(|par| format!(" ∈ {}", par.name.bright_black()))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let count_str = if *count > 0 {
            format!("  {}", format!("{} event{}", count, if *count == 1 { "" } else { "s" }).yellow())
        } else {
            format!("  {}", "no events".bright_black())
        };
        out.push_str(&format!(
            "  {} {}{}{}{}\n",
            fmt_id(&place.id.to_string()),
            place.name.bold(),
            parent_str,
            coord_str,
            count_str,
        ));
    }
    Ok(out)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// ─── global timeline report ─────────────────────────────────────────────────

/// Chronological timeline of all events across the database, with person names.
pub fn global_timeline_report(db: &Database, limit: usize) -> KinforgeResult<String> {
    let mut all_events = db.list_all_events()?;
    let mut out = String::new();

    // Sort: dated events chronologically, undated at end
    all_events.sort_by(|a, b| {
        let key = |e: &Event| -> Option<NaiveDate> {
            e.date.as_ref().and_then(|d| match d {
                EventDate::Exact(nd) | EventDate::Approximate(nd)
                | EventDate::Before(nd) | EventDate::After(nd)
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

    let total = all_events.len();
    let shown = total.min(limit);

    out.push_str(&format!(
        "{}\n{}\n\n",
        format!("  Global Timeline ({} events, showing {})  ", total, shown)
            .bold().bright_cyan().on_black(),
        "─".repeat(44).bright_black()
    ));

    for e in all_events.iter().take(limit) {
        let person_name = db.get_person(&e.person_id)
            .map(|p| p.display_name())
            .unwrap_or_else(|_| e.person_id.to_string());
        let date_str = e.date.as_ref()
            .map(|d| d.to_string().yellow().to_string())
            .unwrap_or_else(|| "undated".bright_black().to_string());
        let place_str = e.place_id.as_ref()
            .and_then(|pid| db.get_place(pid).ok())
            .map(|pl| format!(" @ {}", pl.name.green()))
            .unwrap_or_default();
        out.push_str(&format!(
            "  {:<12} {}  {}  {}{}\n",
            e.event_type.to_string().bright_cyan(),
            date_str,
            person_name.bold(),
            "".to_string(),
            place_str
        ));
    }

    if total > limit {
        out.push_str(&format!(
            "\n  {} more events not shown (use --limit to increase)\n",
            (total - limit).to_string().bright_black()
        ));
    }

    Ok(out)
}

// ─── summary report ─────────────────────────────────────────────────────────

/// One-page compact summary: counts, completeness metrics, top surnames, top event types.
pub fn summary_report(db: &Database) -> KinforgeResult<String> {
    let people = db.list_people()?;
    let all_events = db.list_all_events()?;
    let sources = db.list_sources()?;
    let places = db.list_places()?;
    let stats = db.stats()?;

    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "{}\n{}\n\n",
        "  Family History Summary  ".bold().bright_cyan().on_black(),
        "─".repeat(30).bright_black()
    ));

    // Record counts
    let rows: &[(&str, u64)] = &[
        ("People", stats.people),
        ("Events", stats.events),
        ("Relationships", stats.relationships),
        ("Places", stats.places),
        ("Sources", stats.sources),
        ("Citations", stats.citations),
    ];
    for (label, count) in rows {
        out.push_str(&format!(
            "  {:<22} {}\n",
            label.cyan(),
            count.to_string().bold().yellow()
        ));
    }

    // Derived metrics
    if stats.people > 0 {
        let avg = stats.events as f64 / stats.people as f64;
        out.push_str(&format!(
            "  {:<22} {}\n",
            "Avg events / person".cyan(),
            format!("{:.1}", avg).bold()
        ));
    }
    if stats.events > 0 {
        let pct = stats.citations * 100 / stats.events;
        out.push_str(&format!(
            "  {:<22} {}\n",
            "Citation coverage".cyan(),
            format!("{}%", pct).bold()
        ));
    }
    let _ = places;

    // Top 5 surnames
    let mut surname_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for p in &people {
        if let Some(sn) = p.names.first().and_then(|n| n.surname.as_deref()) {
            if !sn.is_empty() {
                *surname_counts.entry(sn.to_string()).or_insert(0) += 1;
            }
        }
    }
    if !surname_counts.is_empty() {
        out.push_str(&format!("\n{}\n", "Top Surnames:".bold().cyan()));
        let mut surnames: Vec<(&String, &usize)> = surname_counts.iter().collect();
        surnames.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (name, count) in surnames.iter().take(5) {
            out.push_str(&format!(
                "  {:>4}  {}\n",
                count.to_string().yellow().bold(),
                name.bold()
            ));
        }
    }

    // Top 5 event types
    let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in &all_events {
        *type_counts.entry(e.event_type.to_string()).or_insert(0) += 1;
    }
    if !type_counts.is_empty() {
        out.push_str(&format!("\n{}\n", "Top Event Types:".bold().cyan()));
        let mut types: Vec<(&String, &usize)> = type_counts.iter().collect();
        types.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (etype, count) in types.iter().take(5) {
            out.push_str(&format!(
                "  {:>4}  {}\n",
                count.to_string().yellow().bold(),
                etype.bold()
            ));
        }
    }

    // Source overview
    if !sources.is_empty() {
        out.push_str(&format!(
            "\n{} {}\n",
            "Sources:".cyan().bold(),
            format!("{} record(s)", sources.len()).bright_black()
        ));
        for s in sources.iter().take(5) {
            out.push_str(&format!("  • {}\n", s.title.bold()));
        }
        if sources.len() > 5 {
            out.push_str(&format!(
                "  {} more…\n",
                (sources.len() - 5).to_string().bright_black()
            ));
        }
    }

    Ok(out)
}

// ─── birthdays report ────────────────────────────────────────────────────────

/// Annual birthday reference: all people with known birth month+day, sorted by month then day.
pub fn birthdays_report(db: &Database) -> KinforgeResult<String> {
    use chrono::Datelike;

    let people = db.list_people()?;
    let mut entries: Vec<(u32, u32, i32, String, Option<String>)> = Vec::new();
    // (month, day, birth_year, display_name, place_name)

    for person in &people {
        let events = db.list_events_for_person(&person.id)?;
        if let Some(birth) = events.iter().find(|e| matches!(e.event_type, EventType::Birth)) {
            if let Some(nd) = birth.date.as_ref().and_then(|d| match d {
                EventDate::Exact(nd) | EventDate::Approximate(nd) => Some(*nd),
                _ => None,
            }) {
                let place_name = birth.place_id.as_ref()
                    .and_then(|pid| db.get_place(pid).ok())
                    .map(|pl| pl.name);
                entries.push((nd.month(), nd.day(), nd.year(), person.display_name(), place_name));
            }
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    let month_names = [
        "", "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];

    let mut out = String::new();
    out.push_str(&format!(
        "{}\n{}\n\n",
        format!("  Birthdays ({} with known date)  ", entries.len())
            .bold().bright_cyan().on_black(),
        "─".repeat(38).bright_black()
    ));

    if entries.is_empty() {
        out.push_str(&format!("  {}\n", "(no birth dates recorded)".bright_black()));
        return Ok(out);
    }

    let mut current_month = 0u32;
    for (month, day, year, name, place) in &entries {
        if *month != current_month {
            current_month = *month;
            out.push_str(&format!("\n  {}\n", month_names[*month as usize].bold().cyan()));
        }
        let place_str = place.as_deref()
            .map(|p| format!("  @ {}", p.green()))
            .unwrap_or_default();
        out.push_str(&format!(
            "    {:>2}  {}  {}{}\n",
            day.to_string().yellow().bold(),
            name.bold(),
            year.to_string().bright_black(),
            place_str
        ));
    }

    Ok(out)
}
