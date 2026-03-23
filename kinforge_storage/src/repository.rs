use std::path::Path;

use kinforge_core::{
    models::*,
    KinforgeError, KinforgeResult,
};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::migrations::run_migrations;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> KinforgeResult<Self> {
        let conn = Connection::open(path).map_err(|e| KinforgeError::Storage(e.to_string()))?;
        run_migrations(&conn).map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> KinforgeResult<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        run_migrations(&conn).map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(Self { conn })
    }

    // ── People ───────────────────────────────────────────────────────────────

    pub fn insert_person(&self, person: &Person) -> KinforgeResult<()> {
        self.conn
            .execute(
                "INSERT INTO people (id, sex, notes) VALUES (?1, ?2, ?3)",
                params![
                    person.id.as_str(),
                    person.sex.to_string(),
                    person.notes,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        for (i, name) in person.names.iter().enumerate() {
            self.conn
                .execute(
                    "INSERT INTO person_names (id, person_id, name_type, given, surname, prefix, suffix, sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        Uuid::new_v4().to_string(),
                        person.id.as_str(),
                        name.name_type.to_string(),
                        name.given,
                        name.surname,
                        name.prefix,
                        name.suffix,
                        i as i64,
                    ],
                )
                .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        }
        Ok(())
    }

    pub fn get_person(&self, id: &PersonId) -> KinforgeResult<Person> {
        let person = self
            .conn
            .query_row(
                "SELECT id, sex, notes FROM people WHERE id = ?1",
                params![id.as_str()],
                |row| {
                    let id_str: String = row.get(0)?;
                    let sex_str: String = row.get(1)?;
                    let notes: Option<String> = row.get(2)?;
                    Ok((id_str, sex_str, notes))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => KinforgeError::NotFound {
                    entity_type: "Person".to_string(),
                    id: id.as_str(),
                },
                other => KinforgeError::Storage(other.to_string()),
            })?;

        let person_id = PersonId::from_str(&person.0)
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        let sex: Sex = person.1.parse().unwrap_or(Sex::Unknown);
        let names = self.get_names_for_person(&person_id)?;

        Ok(Person {
            id: person_id,
            names,
            sex,
            notes: person.2,
        })
    }

    fn get_names_for_person(&self, person_id: &PersonId) -> KinforgeResult<Vec<PersonName>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT name_type, given, surname, prefix, suffix
                 FROM person_names WHERE person_id = ?1 ORDER BY sort_order",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let names = stmt
            .query_map(params![person_id.as_str()], |row| {
                let name_type_str: String = row.get(0)?;
                Ok(PersonName {
                    name_type: name_type_str
                        .parse()
                        .unwrap_or(NameType::Birth),
                    given: row.get(1)?,
                    surname: row.get(2)?,
                    prefix: row.get(3)?,
                    suffix: row.get(4)?,
                })
            })
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        Ok(names)
    }

    pub fn list_people(&self) -> KinforgeResult<Vec<Person>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM people ORDER BY rowid")
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        ids.iter()
            .map(|id| {
                let pid = PersonId::from_str(id)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_person(&pid)
            })
            .collect()
    }

    pub fn delete_person(&self, id: &PersonId) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM people WHERE id = ?1",
                params![id.as_str()],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Person".to_string(),
                id: id.as_str(),
            });
        }
        Ok(())
    }

    // ── Events ───────────────────────────────────────────────────────────────

    pub fn insert_event(&self, event: &Event) -> KinforgeResult<()> {
        let (date_kind, date_val, date_val2) = match &event.date {
            Some(d) => (Some(d.kind_str().to_string()), d.date_str(), d.date2_str()),
            None => (None, None, None),
        };

        self.conn
            .execute(
                "INSERT INTO events (id, person_id, event_type, date_kind, date_value, date_value2, place_id, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    event.id.as_str(),
                    event.person_id.as_str(),
                    event.event_type.to_string(),
                    date_kind,
                    date_val,
                    date_val2,
                    event.place_id.as_ref().map(|p| p.as_str()),
                    event.notes,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn get_event(&self, id: &EventId) -> KinforgeResult<Event> {
        self.conn
            .query_row(
                "SELECT id, person_id, event_type, date_kind, date_value, date_value2, place_id, notes
                 FROM events WHERE id = ?1",
                params![id.as_str()],
                |row| {
                    let id_str: String = row.get(0)?;
                    let person_id_str: String = row.get(1)?;
                    let event_type_str: String = row.get(2)?;
                    let date_kind: Option<String> = row.get(3)?;
                    let date_val: Option<String> = row.get(4)?;
                    let date_val2: Option<String> = row.get(5)?;
                    let place_id_str: Option<String> = row.get(6)?;
                    let notes: Option<String> = row.get(7)?;
                    Ok((id_str, person_id_str, event_type_str, date_kind, date_val, date_val2, place_id_str, notes))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => KinforgeError::NotFound {
                    entity_type: "Event".to_string(),
                    id: id.as_str(),
                },
                other => KinforgeError::Storage(other.to_string()),
            })
            .and_then(|(id_str, pid_str, et_str, dk, dv, dv2, place_str, notes)| {
                let eid = EventId::from_str(&id_str)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                let pid = PersonId::from_str(&pid_str)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                let event_type: EventType = et_str.parse().unwrap_or(EventType::Other(et_str.clone()));
                let date = dk
                    .as_deref()
                    .and_then(|k| EventDate::from_parts(k, dv.as_deref(), dv2.as_deref()));
                let place_id = place_str
                    .as_deref()
                    .and_then(|s| PlaceId::from_str(s).ok());
                Ok(Event { id: eid, person_id: pid, event_type, date, place_id, notes })
            })
    }

    pub fn list_events_for_person(&self, person_id: &PersonId) -> KinforgeResult<Vec<Event>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM events WHERE person_id = ?1 ORDER BY rowid")
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map(params![person_id.as_str()], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        ids.iter()
            .map(|id| {
                let eid = EventId::from_str(id)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_event(&eid)
            })
            .collect()
    }

    pub fn list_all_events(&self) -> KinforgeResult<Vec<Event>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM events ORDER BY rowid")
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        ids.iter()
            .map(|id| {
                let eid = EventId::from_str(id)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_event(&eid)
            })
            .collect()
    }

    // ── Places ───────────────────────────────────────────────────────────────

    pub fn insert_place(&self, place: &Place) -> KinforgeResult<()> {
        self.conn
            .execute(
                "INSERT INTO places (id, name, latitude, longitude, parent_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    place.id.as_str(),
                    place.name,
                    place.latitude,
                    place.longitude,
                    place.parent_id.as_ref().map(|p| p.as_str()),
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn get_place(&self, id: &PlaceId) -> KinforgeResult<Place> {
        self.conn
            .query_row(
                "SELECT id, name, latitude, longitude, parent_id FROM places WHERE id = ?1",
                params![id.as_str()],
                |row| {
                    let id_str: String = row.get(0)?;
                    let name: String = row.get(1)?;
                    let lat: Option<f64> = row.get(2)?;
                    let lon: Option<f64> = row.get(3)?;
                    let parent_str: Option<String> = row.get(4)?;
                    Ok((id_str, name, lat, lon, parent_str))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => KinforgeError::NotFound {
                    entity_type: "Place".to_string(),
                    id: id.as_str(),
                },
                other => KinforgeError::Storage(other.to_string()),
            })
            .and_then(|(id_str, name, lat, lon, parent_str)| {
                let pid = PlaceId::from_str(&id_str)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                let parent_id = parent_str
                    .as_deref()
                    .and_then(|s| PlaceId::from_str(s).ok());
                Ok(Place { id: pid, name, latitude: lat, longitude: lon, parent_id })
            })
    }

    pub fn list_places(&self) -> KinforgeResult<Vec<Place>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM places ORDER BY name")
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        ids.iter()
            .map(|id| {
                let pid = PlaceId::from_str(id)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_place(&pid)
            })
            .collect()
    }

    // ── Relationships ─────────────────────────────────────────────────────────

    pub fn insert_relationship(&self, rel: &Relationship) -> KinforgeResult<()> {
        self.conn
            .execute(
                "INSERT INTO relationships (id, rel_type, person1_id, person2_id, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    rel.id.as_str(),
                    rel.rel_type.to_string(),
                    rel.person1_id.as_str(),
                    rel.person2_id.as_str(),
                    rel.notes,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn get_relationship(&self, id: &RelationshipId) -> KinforgeResult<Relationship> {
        self.conn
            .query_row(
                "SELECT id, rel_type, person1_id, person2_id, notes FROM relationships WHERE id = ?1",
                params![id.as_str()],
                |row| {
                    let id_str: String = row.get(0)?;
                    let rt_str: String = row.get(1)?;
                    let p1_str: String = row.get(2)?;
                    let p2_str: String = row.get(3)?;
                    let notes: Option<String> = row.get(4)?;
                    Ok((id_str, rt_str, p1_str, p2_str, notes))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => KinforgeError::NotFound {
                    entity_type: "Relationship".to_string(),
                    id: id.as_str(),
                },
                other => KinforgeError::Storage(other.to_string()),
            })
            .and_then(|(id_str, rt_str, p1_str, p2_str, notes)| {
                let rid = RelationshipId::from_str(&id_str)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                let p1 = PersonId::from_str(&p1_str)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                let p2 = PersonId::from_str(&p2_str)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                let rel_type: RelationshipType = rt_str
                    .parse()
                    .map_err(|e: KinforgeError| KinforgeError::Storage(e.to_string()))?;
                Ok(Relationship { id: rid, rel_type, person1_id: p1, person2_id: p2, notes })
            })
    }

    pub fn list_relationships_for_person(&self, person_id: &PersonId) -> KinforgeResult<Vec<Relationship>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id FROM relationships WHERE person1_id = ?1 OR person2_id = ?1 ORDER BY rowid",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map(params![person_id.as_str()], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        ids.iter()
            .map(|id| {
                let rid = RelationshipId::from_str(id)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_relationship(&rid)
            })
            .collect()
    }

    pub fn list_all_relationships(&self) -> KinforgeResult<Vec<Relationship>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM relationships ORDER BY rowid")
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        ids.iter()
            .map(|id| {
                let rid = RelationshipId::from_str(id)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_relationship(&rid)
            })
            .collect()
    }

    // ── Sources ───────────────────────────────────────────────────────────────

    pub fn insert_source(&self, source: &Source) -> KinforgeResult<()> {
        self.conn
            .execute(
                "INSERT INTO sources (id, title, author, publication, year, repository, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    source.id.as_str(),
                    source.title,
                    source.author,
                    source.publication,
                    source.year,
                    source.repository,
                    source.notes,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn get_source(&self, id: &SourceId) -> KinforgeResult<Source> {
        self.conn
            .query_row(
                "SELECT id, title, author, publication, year, repository, notes FROM sources WHERE id = ?1",
                params![id.as_str()],
                |row| {
                    let id_str: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let author: Option<String> = row.get(2)?;
                    let publication: Option<String> = row.get(3)?;
                    let year: Option<i32> = row.get(4)?;
                    let repository: Option<String> = row.get(5)?;
                    let notes: Option<String> = row.get(6)?;
                    Ok((id_str, title, author, publication, year, repository, notes))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => KinforgeError::NotFound {
                    entity_type: "Source".to_string(),
                    id: id.as_str(),
                },
                other => KinforgeError::Storage(other.to_string()),
            })
            .and_then(|(id_str, title, author, publication, year, repository, notes)| {
                let sid = SourceId::from_str(&id_str)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                Ok(Source { id: sid, title, author, publication, year, repository, notes })
            })
    }

    pub fn list_sources(&self) -> KinforgeResult<Vec<Source>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM sources ORDER BY title")
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        ids.iter()
            .map(|id| {
                let sid = SourceId::from_str(id)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_source(&sid)
            })
            .collect()
    }

    // ── Citations ─────────────────────────────────────────────────────────────

    pub fn insert_citation(&self, citation: &Citation) -> KinforgeResult<()> {
        self.conn
            .execute(
                "INSERT INTO citations (id, source_id, event_id, page, confidence, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    citation.id.as_str(),
                    citation.source_id.as_str(),
                    citation.event_id.as_str(),
                    citation.page,
                    citation.confidence.to_string(),
                    citation.notes,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn get_citation(&self, id: &CitationId) -> KinforgeResult<Citation> {
        self.conn
            .query_row(
                "SELECT id, source_id, event_id, page, confidence, notes FROM citations WHERE id = ?1",
                params![id.as_str()],
                |row| {
                    let id_str: String = row.get(0)?;
                    let source_id_str: String = row.get(1)?;
                    let event_id_str: String = row.get(2)?;
                    let page: Option<String> = row.get(3)?;
                    let confidence_str: String = row.get(4)?;
                    let notes: Option<String> = row.get(5)?;
                    Ok((id_str, source_id_str, event_id_str, page, confidence_str, notes))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => KinforgeError::NotFound {
                    entity_type: "Citation".to_string(),
                    id: id.as_str(),
                },
                other => KinforgeError::Storage(other.to_string()),
            })
            .and_then(|(id_str, sid_str, eid_str, page, conf_str, notes)| {
                let cid = CitationId::from_str(&id_str)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                let sid = SourceId::from_str(&sid_str)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                let eid = EventId::from_str(&eid_str)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                let confidence: ConfidenceLevel = conf_str
                    .parse()
                    .unwrap_or(ConfidenceLevel::Secondary);
                Ok(Citation { id: cid, source_id: sid, event_id: eid, page, confidence, notes })
            })
    }

    pub fn list_citations_for_event(&self, event_id: &EventId) -> KinforgeResult<Vec<Citation>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM citations WHERE event_id = ?1 ORDER BY rowid")
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map(params![event_id.as_str()], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        ids.iter()
            .map(|id| {
                let cid = CitationId::from_str(id)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_citation(&cid)
            })
            .collect()
    }

    pub fn list_all_citations(&self) -> KinforgeResult<Vec<Citation>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM citations ORDER BY rowid")
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        ids.iter()
            .map(|id| {
                let cid = CitationId::from_str(id)
                    .map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_citation(&cid)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let mut person = Person::new(Sex::Male);
        person.names.push(PersonName {
            given: Some("John".to_string()),
            surname: Some("Smith".to_string()),
            name_type: NameType::Birth,
            prefix: None,
            suffix: None,
        });

        db.insert_person(&person).unwrap();
        let fetched = db.get_person(&person.id).unwrap();
        assert_eq!(fetched.id, person.id);
        assert_eq!(fetched.sex, Sex::Male);
        assert_eq!(fetched.names[0].given, Some("John".to_string()));
    }

    #[test]
    fn test_event_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let person = Person::new(Sex::Female);
        db.insert_person(&person).unwrap();

        let mut event = Event::new(EventType::Birth, person.id.clone());
        event.notes = Some("Hospital birth".to_string());
        db.insert_event(&event).unwrap();

        let fetched = db.get_event(&event.id).unwrap();
        assert_eq!(fetched.id, event.id);
        assert_eq!(fetched.event_type, EventType::Birth);
    }

    #[test]
    fn test_source_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let mut source = Source::new("1900 US Census");
        source.author = Some("US Bureau of the Census".to_string());
        source.year = Some(1900);

        db.insert_source(&source).unwrap();
        let fetched = db.get_source(&source.id).unwrap();
        assert_eq!(fetched.title, "1900 US Census");
        assert_eq!(fetched.year, Some(1900));
    }
}
