use kinforge_core::{models::*, KinforgeError, KinforgeResult};
use kinforge_storage::Database;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use crate::ImportStats;

#[derive(Serialize, Deserialize)]
struct KinforgeExport {
    version: String,
    people: Vec<Person>,
    events: Vec<Event>,
    places: Vec<Place>,
    relationships: Vec<Relationship>,
    sources: Vec<Source>,
    citations: Vec<Citation>,
}

pub fn export_json<W: Write>(db: &Database, writer: &mut W) -> KinforgeResult<()> {
    let export = KinforgeExport {
        version: "0.1.0".to_string(),
        people: db.list_people()?,
        events: db.list_all_events()?,
        places: db.list_places()?,
        relationships: db.list_all_relationships()?,
        sources: db.list_sources()?,
        citations: db.list_all_citations()?,
    };

    serde_json::to_writer_pretty(writer, &export)
        .map_err(|e| KinforgeError::ImportExport(e.to_string()))
}

pub fn import_json<R: Read>(db: &Database, reader: &mut R) -> KinforgeResult<ImportStats> {
    let mut content = String::new();
    reader.read_to_string(&mut content)?;

    let export: KinforgeExport =
        serde_json::from_str(&content).map_err(|e| KinforgeError::ImportExport(e.to_string()))?;

    let mut stats = ImportStats::default();

    for place in &export.places {
        db.insert_place(place)?;
        stats.places += 1;
    }
    for person in &export.people {
        db.insert_person(person)?;
        stats.people += 1;
    }
    for source in &export.sources {
        db.insert_source(source)?;
        stats.sources += 1;
    }
    for event in &export.events {
        db.insert_event(event)?;
        stats.events += 1;
    }
    for rel in &export.relationships {
        db.insert_relationship(rel)?;
        stats.relationships += 1;
    }
    for citation in &export.citations {
        db.insert_citation(citation)?;
    }

    Ok(stats)
}
