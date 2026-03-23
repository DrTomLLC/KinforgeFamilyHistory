use kinforge_core::{models::*, KinforgeError, KinforgeResult};
use kinforge_storage::Database;
use std::collections::HashMap;

/// Import a GEDCOM 5.5 file into the database.
pub fn import_gedcom(content: &str, db: &Database) -> KinforgeResult<ImportStats> {
    let records = parse_gedcom(content)?;
    let mut stats = ImportStats::default();

    // First pass: create people
    let mut person_map: HashMap<String, PersonId> = HashMap::new();
    for (tag, record_id, lines) in &records {
        if tag == "INDI" {
            let person = parse_individual(lines)?;
            let gedcom_id = record_id.clone();
            let person_id = person.id.clone();
            db.insert_person(&person)?;
            person_map.insert(gedcom_id, person_id);
            stats.people += 1;
        }
    }

    // Second pass: create sources
    let mut source_map: HashMap<String, SourceId> = HashMap::new();
    for (tag, record_id, lines) in &records {
        if tag == "SOUR" {
            let source = parse_source(lines)?;
            let gedcom_id = record_id.clone();
            let source_id = source.id.clone();
            db.insert_source(&source)?;
            source_map.insert(gedcom_id, source_id);
            stats.sources += 1;
        }
    }

    Ok(stats)
}

#[derive(Debug, Default)]
pub struct ImportStats {
    pub people: usize,
    pub sources: usize,
    pub events: usize,
}

/// Parse GEDCOM into top-level records: (tag, record_id, lines)
fn parse_gedcom(content: &str) -> KinforgeResult<Vec<(String, String, Vec<GedcomLine>)>> {
    let mut records = Vec::new();
    let mut current_record: Option<(String, String, Vec<GedcomLine>)> = None;
    let mut counter = 0u32;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let parsed = parse_line(line)?;

        if parsed.level == 0 {
            if let Some(record) = current_record.take() {
                records.push(record);
            }
            if parsed.tag == "HEAD" || parsed.tag == "TRLR" {
                continue;
            }
            counter += 1;
            let record_id = parsed
                .xref_id
                .clone()
                .unwrap_or_else(|| format!("AUTO{}", counter));
            current_record = Some((parsed.tag.clone(), record_id, vec![parsed]));
        } else if let Some(ref mut record) = current_record {
            record.2.push(parsed);
        }
    }
    if let Some(record) = current_record.take() {
        records.push(record);
    }
    Ok(records)
}

#[derive(Debug, Clone)]
struct GedcomLine {
    level: u8,
    xref_id: Option<String>,
    tag: String,
    value: Option<String>,
}

fn parse_line(line: &str) -> KinforgeResult<GedcomLine> {
    let mut parts = line.splitn(3, ' ');
    let level_str = parts.next().unwrap_or("0");
    let level: u8 = level_str
        .parse()
        .map_err(|_| KinforgeError::ImportExport(format!("Invalid GEDCOM level: {}", level_str)))?;

    let second = parts.next().unwrap_or("").trim();
    let rest = parts.next().map(|s| s.trim().to_string());

    let (xref_id, tag) = if second.starts_with('@') && second.ends_with('@') {
        // This is an xref_id followed by tag in rest
        let id = second[1..second.len() - 1].to_string();
        let tag_str = rest
            .as_deref()
            .and_then(|r| r.split_whitespace().next())
            .unwrap_or("")
            .to_string();
        (Some(id), tag_str)
    } else {
        (None, second.to_string())
    };

    // If xref_id was parsed, the value is the part after the tag
    let value = if xref_id.is_some() {
        rest.as_deref()
            .and_then(|r| {
                let mut p = r.splitn(2, ' ');
                p.next(); // skip tag
                p.next().map(|s| s.to_string())
            })
    } else {
        rest
    };

    Ok(GedcomLine {
        level,
        xref_id,
        tag,
        value,
    })
}

fn parse_individual(lines: &[GedcomLine]) -> KinforgeResult<Person> {
    let mut person = Person::new(Sex::Unknown);
    let mut i = 1usize; // skip level-0 line

    while i < lines.len() {
        let line = &lines[i];
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
                        let name = parse_gedcom_name(val);
                        person.names.push(name);
                    }
                }
                "NOTE" => {
                    person.notes = line.value.clone();
                }
                _ => {}
            }
        }
        i += 1;
    }
    Ok(person)
}

fn parse_gedcom_name(val: &str) -> PersonName {
    // Format: "Given /Surname/ Suffix"
    let mut given = None;
    let mut surname = None;

    if let (Some(start), Some(end)) = (val.find('/'), val.rfind('/')) {
        if start != end {
            let s = val[start + 1..end].trim();
            if !s.is_empty() {
                surname = Some(s.to_string());
            }
            let g = val[..start].trim();
            if !g.is_empty() {
                given = Some(g.to_string());
            }
        }
    } else {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            given = Some(trimmed.to_string());
        }
    }

    PersonName {
        given,
        surname,
        name_type: NameType::Birth,
        prefix: None,
        suffix: None,
    }
}

fn parse_source(lines: &[GedcomLine]) -> KinforgeResult<Source> {
    let mut source = Source::new("Untitled Source");

    for line in lines.iter().skip(1) {
        if line.level == 1 {
            match line.tag.as_str() {
                "TITL" => {
                    source.title = line.value.clone().unwrap_or_default();
                }
                "AUTH" => {
                    source.author = line.value.clone();
                }
                "PUBL" => {
                    source.publication = line.value.clone();
                }
                _ => {}
            }
        }
    }
    Ok(source)
}
