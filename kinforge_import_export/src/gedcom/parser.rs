use chrono::NaiveDate;
use kinforge_core::{models::*, KinforgeError, KinforgeResult};
use kinforge_storage::Database;
use std::collections::{HashMap, HashSet};

// ── Public entry point ────────────────────────────────────────────────────────

/// Import a GEDCOM 5.5 file into the database.
pub fn import_gedcom(content: &str, db: &Database) -> KinforgeResult<ImportStats> {
    let records = parse_top_level(content)?;
    let mut stats = ImportStats::default();

    // Map gedcom xref → internal ID for cross-references.
    let mut person_map: HashMap<String, PersonId> = HashMap::new();
    let mut source_map: HashMap<String, SourceId> = HashMap::new();

    // Build a fingerprint set from people already in the DB so we can skip duplicates.
    // Fingerprint: lowercase(display_name) + birth_year (or "?")
    let mut seen_fingerprints: HashSet<String> = db
        .list_people()
        .unwrap_or_default()
        .iter()
        .map(|p| {
            let birth_year = db
                .list_events_for_person(&p.id)
                .ok()
                .and_then(|evts| {
                    evts.into_iter()
                        .find(|e| matches!(e.event_type, EventType::Birth))
                        .and_then(|e| e.date)
                        .and_then(|d| match d {
                            EventDate::Exact(nd) | EventDate::Approximate(nd) => {
                                Some(nd.format("%Y").to_string())
                            }
                            _ => None,
                        })
                })
                .unwrap_or_else(|| "?".to_string());
            format!("{}|{}", p.display_name().to_lowercase(), birth_year)
        })
        .collect();

    // Pass 1: individuals
    for rec in &records {
        if rec.tag == "INDI" {
            let (person, events) = parse_individual_record(rec)?;

            // Compute fingerprint for duplicate check
            let birth_year = events
                .iter()
                .find(|pe| matches!(pe.event.event_type, EventType::Birth))
                .and_then(|pe| pe.event.date.as_ref())
                .and_then(|d| match d {
                    EventDate::Exact(nd) | EventDate::Approximate(nd) => {
                        Some(nd.format("%Y").to_string())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| "?".to_string());
            let fp = format!("{}|{}", person.display_name().to_lowercase(), birth_year);

            if seen_fingerprints.contains(&fp) {
                stats.duplicates_skipped += 1;
                // Still need a mapping so family records don't error — skip insertion only
                person_map.insert(rec.xref_id.clone(), person.id.clone());
                continue;
            }
            seen_fingerprints.insert(fp);

            person_map.insert(rec.xref_id.clone(), person.id.clone());
            db.insert_person(&person)?;
            stats.people += 1;

            for mut event in events {
                if let Some(ref place_name) = event.pending_place {
                    let place = Place::new(place_name.clone());
                    db.insert_place(&place)?;
                    event.event.place_id = Some(place.id);
                }
                db.insert_event(&event.event)?;
                stats.events += 1;
            }
        }
    }

    // Pass 2: sources
    for rec in &records {
        if rec.tag == "SOUR" {
            let source = parse_source_record(rec)?;
            source_map.insert(rec.xref_id.clone(), source.id.clone());
            db.insert_source(&source)?;
            stats.sources += 1;
        }
    }

    // Pass 3: family records → relationships
    for rec in &records {
        if rec.tag == "FAM" {
            let rels = parse_family_record(rec, &person_map)?;
            for rel in rels {
                db.insert_relationship(&rel)?;
                stats.relationships += 1;
            }
        }
    }

    Ok(stats)
}

#[derive(Debug, Default)]
pub struct ImportStats {
    pub people: usize,
    pub events: usize,
    pub sources: usize,
    pub relationships: usize,
    pub duplicates_skipped: usize,
}

// ── GEDCOM record / line structures ──────────────────────────────────────────

#[derive(Debug)]
struct GedcomRecord {
    /// Xref identifier without @, e.g. "I1"
    xref_id: String,
    /// Tag, e.g. "INDI", "FAM", "SOUR"
    tag: String,
    /// All lines belonging to this record (level ≥ 0)
    lines: Vec<GedcomLine>,
}

#[derive(Debug, Clone)]
struct GedcomLine {
    level: u8,
    tag: String,
    value: Option<String>,
}

// ── Top-level parser ──────────────────────────────────────────────────────────

fn parse_top_level(content: &str) -> KinforgeResult<Vec<GedcomRecord>> {
    let mut records: Vec<GedcomRecord> = Vec::new();
    let mut current: Option<GedcomRecord> = None;
    let mut counter = 0u32;

    for raw in content.lines() {
        let line_str = raw.trim();
        if line_str.is_empty() {
            continue;
        }
        let gl = parse_line(line_str)?;

        if gl.level == 0 {
            if let Some(rec) = current.take() {
                records.push(rec);
            }
            if gl.tag == "HEAD" || gl.tag == "TRLR" {
                continue;
            }
            // value field on a level-0 line is the tag (xref comes first)
            counter += 1;
            let xref = gl.xref_id.unwrap_or_else(|| format!("AUTO{}", counter));
            let tag = gl.tag;
            current = Some(GedcomRecord {
                xref_id: xref,
                tag,
                lines: Vec::new(),
            });
        } else if let Some(ref mut rec) = current {
            rec.lines.push(GedcomLine {
                level: gl.level,
                tag: gl.tag,
                value: gl.value,
            });
        }
    }
    if let Some(rec) = current {
        records.push(rec);
    }
    Ok(records)
}

// Raw parse output before xref/tag are separated for level-0 lines.
struct RawLine {
    level: u8,
    xref_id: Option<String>,
    tag: String,
    value: Option<String>,
}

fn parse_line(s: &str) -> KinforgeResult<RawLine> {
    let mut parts = s.splitn(4, ' ');
    let level_str = parts.next().unwrap_or("0");
    let level: u8 = level_str
        .parse()
        .map_err(|_| KinforgeError::ImportExport(format!("Bad GEDCOM level: {}", level_str)))?;

    let second = parts.next().unwrap_or("").trim();
    let third = parts.next().map(|s| s.trim().to_string());
    let fourth = parts.next().map(|s| s.trim().to_string());

    // Level-0 lines: `0 @XREF@ TAG [value]`
    if level == 0 && second.starts_with('@') && second.ends_with('@') {
        let xref = second[1..second.len() - 1].to_string();
        let tag = third.clone().unwrap_or_default();
        let value = fourth;
        return Ok(RawLine {
            level,
            xref_id: Some(xref),
            tag,
            value,
        });
    }

    // Normal lines: `N TAG [value]`
    let tag = second.to_string();
    // Re-join third + fourth as value
    let value = match (third, fourth) {
        (None, _) => None,
        (Some(t), None) => Some(t),
        (Some(t), Some(f)) => Some(format!("{} {}", t, f)),
    };
    Ok(RawLine {
        level,
        xref_id: None,
        tag,
        value,
    })
}

// ── Individual parser ─────────────────────────────────────────────────────────

struct PendingEvent {
    event: Event,
    pending_place: Option<String>,
}

fn parse_individual_record(rec: &GedcomRecord) -> KinforgeResult<(Person, Vec<PendingEvent>)> {
    let mut person = Person::new(Sex::Unknown);
    let mut events: Vec<PendingEvent> = Vec::new();

    let mut i = 0usize;
    while i < rec.lines.len() {
        let line = &rec.lines[i];
        if line.level == 1 {
            match line.tag.as_str() {
                "SEX" => {
                    person.sex = match line.value.as_deref() {
                        Some("M") => Sex::Male,
                        Some("F") => Sex::Female,
                        _ => Sex::Unknown,
                    };
                }
                "NAME" => {
                    if let Some(ref val) = line.value {
                        person.names.push(parse_gedcom_name(val));
                    }
                }
                "NOTE" => {
                    person.notes = line.value.clone();
                }
                tag => {
                    if let Some(etype) = gedcom_tag_to_event_type(tag) {
                        // Collect sub-lines for this event
                        let sub_start = i + 1;
                        let mut sub_end = sub_start;
                        while sub_end < rec.lines.len() && rec.lines[sub_end].level > 1 {
                            sub_end += 1;
                        }
                        let sub = &rec.lines[sub_start..sub_end];
                        let pe = parse_event_sublines(etype, person.id.clone(), sub)?;
                        events.push(pe);
                        i = sub_end;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    Ok((person, events))
}

fn parse_event_sublines(
    etype: EventType,
    person_id: PersonId,
    sub: &[GedcomLine],
) -> KinforgeResult<PendingEvent> {
    let mut event = Event::new(etype, person_id);
    let mut pending_place: Option<String> = None;

    for line in sub {
        if line.level == 2 {
            match line.tag.as_str() {
                "DATE" => {
                    event.date = line.value.as_deref().and_then(parse_gedcom_date);
                }
                "PLAC" => {
                    pending_place = line.value.clone();
                }
                "NOTE" => {
                    event.notes = line.value.clone();
                }
                _ => {}
            }
        }
    }
    Ok(PendingEvent {
        event,
        pending_place,
    })
}

fn parse_gedcom_date(s: &str) -> Option<EventDate> {
    let s = s.trim();
    if s.is_empty() || s == "UNKNOWN" {
        return Some(EventDate::Unknown);
    }

    let (prefix, date_str) = if let Some(rest) = s.strip_prefix("ABT ") {
        ("abt", rest.trim())
    } else if let Some(rest) = s.strip_prefix("BEF ") {
        ("bef", rest.trim())
    } else if let Some(rest) = s.strip_prefix("AFT ") {
        ("aft", rest.trim())
    } else if let Some(rest) = s.strip_prefix("BET ") {
        let parts: Vec<&str> = rest.splitn(2, " AND ").collect();
        if parts.len() == 2 {
            let d1 = parse_gedcom_date_value(parts[0].trim())?;
            let d2 = parse_gedcom_date_value(parts[1].trim())?;
            return Some(EventDate::Between(d1, d2));
        }
        return None;
    } else {
        ("exact", s)
    };

    let d = parse_gedcom_date_value(date_str)?;
    match prefix {
        "abt" => Some(EventDate::Approximate(d)),
        "bef" => Some(EventDate::Before(d)),
        "aft" => Some(EventDate::After(d)),
        _ => Some(EventDate::Exact(d)),
    }
}

fn parse_gedcom_date_value(s: &str) -> Option<NaiveDate> {
    // Try various formats: "15 JAN 1900", "JAN 1900", "1900"
    let parts: Vec<&str> = s.split_whitespace().collect();
    match parts.as_slice() {
        [day, month, year] => {
            let m = month_abbr_to_num(month)?;
            let d: u32 = day.parse().ok()?;
            let y: i32 = year.parse().ok()?;
            NaiveDate::from_ymd_opt(y, m, d)
        }
        [month, year] => {
            let m = month_abbr_to_num(month)?;
            let y: i32 = year.parse().ok()?;
            NaiveDate::from_ymd_opt(y, m, 1)
        }
        [year] => {
            let y: i32 = year.parse().ok()?;
            NaiveDate::from_ymd_opt(y, 1, 1)
        }
        _ => None,
    }
}

fn month_abbr_to_num(s: &str) -> Option<u32> {
    match s.to_uppercase().as_str() {
        "JAN" => Some(1),
        "FEB" => Some(2),
        "MAR" => Some(3),
        "APR" => Some(4),
        "MAY" => Some(5),
        "JUN" => Some(6),
        "JUL" => Some(7),
        "AUG" => Some(8),
        "SEP" => Some(9),
        "OCT" => Some(10),
        "NOV" => Some(11),
        "DEC" => Some(12),
        _ => None,
    }
}

fn gedcom_tag_to_event_type(tag: &str) -> Option<EventType> {
    match tag {
        "BIRT" => Some(EventType::Birth),
        "DEAT" => Some(EventType::Death),
        "MARR" => Some(EventType::Marriage),
        "DIV" | "DIVF" => Some(EventType::Divorce),
        "BURI" => Some(EventType::Burial),
        "BAPT" | "CHR" => Some(EventType::Baptism),
        "RESI" => Some(EventType::Residence),
        "OCCU" => Some(EventType::Occupation),
        "EDUC" => Some(EventType::Education),
        "MILI" => Some(EventType::Military),
        "NATU" => Some(EventType::Naturalization),
        "CENS" => Some(EventType::Census),
        "EMIG" => Some(EventType::Emigration),
        "IMMI" => Some(EventType::Immigration),
        "EVEN" => Some(EventType::Other("Event".to_string())),
        _ => None,
    }
}

fn parse_gedcom_name(val: &str) -> PersonName {
    // Format: "Given /Surname/ Suffix" or "Given Surname" or "/Surname/"
    if val.contains('/') {
        let slash1 = val.find('/').unwrap();
        let slash2 = val.rfind('/').unwrap();
        let surname = if slash2 > slash1 + 1 {
            Some(val[slash1 + 1..slash2].trim().to_string())
        } else {
            None
        }
        .filter(|s| !s.is_empty());

        let given = val[..slash1].trim();
        let given = if given.is_empty() {
            None
        } else {
            Some(given.to_string())
        };

        PersonName {
            given,
            surname,
            name_type: NameType::Birth,
            prefix: None,
            suffix: None,
        }
    } else {
        let trimmed = val.trim().to_string();
        if trimmed.is_empty() {
            PersonName {
                given: None,
                surname: None,
                name_type: NameType::Birth,
                prefix: None,
                suffix: None,
            }
        } else {
            PersonName {
                given: Some(trimmed),
                surname: None,
                name_type: NameType::Birth,
                prefix: None,
                suffix: None,
            }
        }
    }
}

// ── Source parser ─────────────────────────────────────────────────────────────

fn parse_source_record(rec: &GedcomRecord) -> KinforgeResult<Source> {
    let mut source = Source::new("Untitled Source");
    for line in &rec.lines {
        if line.level == 1 {
            match line.tag.as_str() {
                "TITL" => {
                    if let Some(ref v) = line.value {
                        source.title = v.clone();
                    }
                }
                "AUTH" => {
                    source.author = line.value.clone();
                }
                "PUBL" => {
                    source.publication = line.value.clone();
                }
                "NOTE" => {
                    source.notes = line.value.clone();
                }
                _ => {}
            }
        }
    }
    Ok(source)
}

// ── Family record parser ──────────────────────────────────────────────────────

fn parse_family_record(
    rec: &GedcomRecord,
    person_map: &HashMap<String, PersonId>,
) -> KinforgeResult<Vec<Relationship>> {
    let mut husb: Option<PersonId> = None;
    let mut wife: Option<PersonId> = None;
    let mut children: Vec<PersonId> = Vec::new();
    let mut rels: Vec<Relationship> = Vec::new();

    for line in &rec.lines {
        if line.level == 1 {
            match line.tag.as_str() {
                "HUSB" => {
                    if let Some(xref) = extract_xref(line.value.as_deref()) {
                        husb = person_map.get(&xref).cloned();
                    }
                }
                "WIFE" => {
                    if let Some(xref) = extract_xref(line.value.as_deref()) {
                        wife = person_map.get(&xref).cloned();
                    }
                }
                "CHIL" => {
                    if let Some(xref) = extract_xref(line.value.as_deref()) {
                        if let Some(cid) = person_map.get(&xref) {
                            children.push(cid.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Spouse relationship
    if let (Some(ref h), Some(ref w)) = (&husb, &wife) {
        rels.push(Relationship::new(
            RelationshipType::Spouse,
            h.clone(),
            w.clone(),
        ));
    }

    // Parent→child relationships
    let parents: Vec<&PersonId> = [husb.as_ref(), wife.as_ref()]
        .into_iter()
        .flatten()
        .collect();
    for child_id in &children {
        for parent_id in &parents {
            rels.push(Relationship::new(
                RelationshipType::ParentChild,
                (*parent_id).clone(),
                child_id.clone(),
            ));
        }
    }

    Ok(rels)
}

/// Extract the xref string from a `@XREF@` value.
fn extract_xref(val: Option<&str>) -> Option<String> {
    let v = val?.trim();
    if v.starts_with('@') && v.ends_with('@') && v.len() > 2 {
        Some(v[1..v.len() - 1].to_string())
    } else {
        None
    }
}
