use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;
use std::collections::HashMap;
use std::io::Write;

/// Export all data to GEDCOM 5.5 format.
pub fn export_gedcom<W: Write>(db: &Database, writer: &mut W) -> KinforgeResult<()> {
    // Write header
    writeln!(writer, "0 HEAD").ok();
    writeln!(writer, "1 SOUR Kinforge").ok();
    writeln!(writer, "2 VERS 0.1.0").ok();
    writeln!(writer, "2 NAME Kinforge Family History").ok();
    writeln!(writer, "1 GEDC").ok();
    writeln!(writer, "2 VERS 5.5").ok();
    writeln!(writer, "1 CHAR UTF-8").ok();

    let people = db.list_people()?;
    let sources = db.list_sources()?;
    let all_rels = db.list_all_relationships()?;

    // Build short GEDCOM IDs: person UUID → "I<n>"
    let person_gid: HashMap<String, String> = people
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), format!("I{}", i + 1)))
        .collect();
    let source_gid: HashMap<String, String> = sources
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), format!("S{}", i + 1)))
        .collect();

    // ── INDI records ──────────────────────────────────────────────────────────
    for person in &people {
        let gid = person_gid.get(&person.id.as_str()).unwrap();
        writeln!(writer, "0 @{}@ INDI", gid).ok();

        for name in &person.names {
            let given = name.given.as_deref().unwrap_or("");
            let surname = name.surname.as_deref().unwrap_or("");
            writeln!(writer, "1 NAME {}/{}/", given, surname).ok();
            if let Some(ref pfx) = name.prefix {
                writeln!(writer, "2 NPFX {}", pfx).ok();
            }
            if let Some(ref sfx) = name.suffix {
                writeln!(writer, "2 NSFX {}", sfx).ok();
            }
        }

        let sex_code = match person.sex {
            Sex::Male => "M",
            Sex::Female => "F",
            Sex::Unknown => "U",
        };
        writeln!(writer, "1 SEX {}", sex_code).ok();

        let events = db.list_events_for_person(&person.id)?;
        for event in &events {
            let tag = event_type_to_gedcom_tag(&event.event_type);
            writeln!(writer, "1 {}", tag).ok();

            if let Some(ref date) = event.date {
                writeln!(writer, "2 DATE {}", gedcom_date_str(date)).ok();
            }
            if let Some(ref place_id) = event.place_id {
                if let Ok(place) = db.get_place(place_id) {
                    writeln!(writer, "2 PLAC {}", place.name).ok();
                }
            }
            if let Some(ref notes) = event.notes {
                writeln!(writer, "2 NOTE {}", notes).ok();
            }

            // Inline citations for this event
            let citations = db.list_citations_for_event(&event.id)?;
            for cit in &citations {
                if let Some(sgid) = source_gid.get(&cit.source_id.as_str()) {
                    writeln!(writer, "2 SOUR @{}@", sgid).ok();
                    if let Some(ref page) = cit.page {
                        writeln!(writer, "3 PAGE {}", page).ok();
                    }
                    writeln!(writer, "3 QUAY {}", confidence_to_quay(&cit.confidence)).ok();
                }
            }
        }

        if let Some(ref notes) = person.notes {
            for line in notes.lines() {
                writeln!(writer, "1 NOTE {}", line).ok();
            }
        }
    }

    // ── FAM records ───────────────────────────────────────────────────────────
    // Group spouse relationships + their children into FAM units
    let mut fam_counter = 0usize;

    // Collect spouse pairs: person1 → person2 (Spouse relationships)
    let spouse_rels: Vec<&Relationship> = all_rels
        .iter()
        .filter(|r| r.rel_type == RelationshipType::Spouse)
        .collect();

    // For each spouse pair, gather their shared children
    for spouse_rel in &spouse_rels {
        fam_counter += 1;
        let p1_gid = person_gid.get(&spouse_rel.person1_id.as_str());
        let p2_gid = person_gid.get(&spouse_rel.person2_id.as_str());

        writeln!(writer, "0 @F{}@ FAM", fam_counter).ok();
        if let Some(gid) = p1_gid {
            // Determine husb/wife by sex
            if let Ok(p) = db.get_person(&spouse_rel.person1_id) {
                let tag = if p.sex == Sex::Female { "WIFE" } else { "HUSB" };
                writeln!(writer, "1 {} @{}@", tag, gid).ok();
            }
        }
        if let Some(gid) = p2_gid {
            if let Ok(p) = db.get_person(&spouse_rel.person2_id) {
                let tag = if p.sex == Sex::Female { "WIFE" } else { "HUSB" };
                writeln!(writer, "1 {} @{}@", tag, gid).ok();
            }
        }

        // Children = people who have EITHER spouse as parent via ParentChild
        let id1 = spouse_rel.person1_id.as_str();
        let id2 = spouse_rel.person2_id.as_str();
        let children1 = children_of(&id1, &all_rels, &person_gid);
        let children2 = children_of(&id2, &all_rels, &person_gid);
        // Intersection: children listed under both parents
        for cgid in &children1 {
            if children2.contains(cgid) {
                writeln!(writer, "1 CHIL @{}@", cgid).ok();
            }
        }
    }

    // Single-parent families: parent→child relationships not already in a FAM
    // Collect all parent IDs that had a spouse relationship
    let paired_parents: std::collections::HashSet<String> = spouse_rels
        .iter()
        .flat_map(|r| [r.person1_id.as_str(), r.person2_id.as_str()])
        .collect();

    for rel in &all_rels {
        if rel.rel_type == RelationshipType::ParentChild {
            let parent_id_str = rel.person1_id.as_str();
            if !paired_parents.contains(&parent_id_str) {
                // Single parent not in a spouse pair
                if let Some(pgid) = person_gid.get(&parent_id_str) {
                    fam_counter += 1;
                    writeln!(writer, "0 @F{}@ FAM", fam_counter).ok();
                    if let Ok(p) = db.get_person(&rel.person1_id) {
                        let tag = if p.sex == Sex::Female { "WIFE" } else { "HUSB" };
                        writeln!(writer, "1 {} @{}@", tag, pgid).ok();
                    }
                    if let Some(cgid) = person_gid.get(&rel.person2_id.as_str()) {
                        writeln!(writer, "1 CHIL @{}@", cgid).ok();
                    }
                }
            }
        }
    }

    // ── SOUR records ──────────────────────────────────────────────────────────
    for source in &sources {
        let gid = source_gid.get(&source.id.as_str()).unwrap();
        writeln!(writer, "0 @{}@ SOUR", gid).ok();
        writeln!(writer, "1 TITL {}", source.title).ok();
        if let Some(ref auth) = source.author {
            writeln!(writer, "1 AUTH {}", auth).ok();
        }
        if let Some(ref publ) = source.publication {
            writeln!(writer, "1 PUBL {}", publ).ok();
        }
        if let Some(year) = source.year {
            writeln!(writer, "1 DATE {}", year).ok();
        }
        if let Some(ref notes) = source.notes {
            writeln!(writer, "1 NOTE {}", notes).ok();
        }
    }

    writeln!(writer, "0 TRLR").ok();
    Ok(())
}

fn children_of(
    parent_id: &str,
    all_rels: &[Relationship],
    person_gid: &HashMap<String, String>,
) -> Vec<String> {
    all_rels
        .iter()
        .filter(|r| {
            r.rel_type == RelationshipType::ParentChild && r.person1_id.as_str() == parent_id
        })
        .filter_map(|r| person_gid.get(&r.person2_id.as_str()).cloned())
        .collect()
}

fn event_type_to_gedcom_tag(et: &EventType) -> &'static str {
    match et {
        EventType::Birth => "BIRT",
        EventType::Death => "DEAT",
        EventType::Marriage => "MARR",
        EventType::Divorce => "DIV",
        EventType::Burial => "BURI",
        EventType::Baptism => "BAPT",
        EventType::Residence => "RESI",
        EventType::Occupation => "OCCU",
        EventType::Education => "EDUC",
        EventType::Military => "MILI",
        EventType::Naturalization => "NATU",
        EventType::Census => "CENS",
        EventType::Emigration => "EMIG",
        EventType::Immigration => "IMMI",
        EventType::Other(_) => "EVEN",
    }
}

fn gedcom_date_str(date: &EventDate) -> String {
    let fmt = |d: &chrono::NaiveDate| d.format("%d %b %Y").to_string().to_uppercase();
    match date {
        EventDate::Exact(d) => fmt(d),
        EventDate::Approximate(d) => format!("ABT {}", fmt(d)),
        EventDate::Before(d) => format!("BEF {}", fmt(d)),
        EventDate::After(d) => format!("AFT {}", fmt(d)),
        EventDate::Between(d1, d2) => format!("BET {} AND {}", fmt(d1), fmt(d2)),
        EventDate::Unknown => "UNKNOWN".to_string(),
    }
}

fn confidence_to_quay(c: &ConfidenceLevel) -> u8 {
    match c {
        ConfidenceLevel::Unreliable => 0,
        ConfidenceLevel::Questionable => 1,
        ConfidenceLevel::Secondary => 2,
        ConfidenceLevel::Primary => 2,
        ConfidenceLevel::Direct => 3,
    }
}
