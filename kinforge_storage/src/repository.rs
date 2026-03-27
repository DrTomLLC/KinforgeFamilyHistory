use std::path::Path;

use kinforge_core::{models::*, KinforgeError, KinforgeResult};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::migrations::run_migrations;

/// Column tuple returned by relationship SQL queries.
type RelRow = (String, String, String, String, Option<String>);

/// A single result from a full-text search.
#[derive(Debug, Clone)]
pub struct FtsResult {
    /// The entity class: "person", "event", "source", or "place"
    pub entity_type: String,
    /// UUID of the matching entity
    pub entity_id: String,
    /// Context snippet from the matching body text (raw, no highlight markers)
    pub snippet: String,
}

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
        let conn =
            Connection::open_in_memory().map_err(|e| KinforgeError::Storage(e.to_string()))?;
        run_migrations(&conn).map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(Self { conn })
    }

    // ── People ───────────────────────────────────────────────────────────────

    pub fn insert_person(&self, person: &Person) -> KinforgeResult<()> {
        self.conn
            .execute(
                "INSERT INTO people (id, sex, notes) VALUES (?1, ?2, ?3)",
                params![person.id.as_str(), person.sex.to_string(), person.notes,],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        self.replace_person_names(&person.id, &person.names)?;
        Ok(())
    }

    pub fn update_person(&self, person: &Person) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute(
                "UPDATE people SET sex = ?2, notes = ?3 WHERE id = ?1",
                params![person.id.as_str(), person.sex.to_string(), person.notes,],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Person".to_string(),
                id: person.id.as_str(),
            });
        }
        self.replace_person_names(&person.id, &person.names)?;
        Ok(())
    }

    fn replace_person_names(
        &self,
        person_id: &PersonId,
        names: &[PersonName],
    ) -> KinforgeResult<()> {
        self.conn
            .execute(
                "DELETE FROM person_names WHERE person_id = ?1",
                params![person_id.as_str()],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        for (i, name) in names.iter().enumerate() {
            self.conn
                .execute(
                    "INSERT INTO person_names (id, person_id, name_type, given, surname, prefix, suffix, sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        Uuid::new_v4().to_string(),
                        person_id.as_str(),
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
        let row = self
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

        let person_id =
            PersonId::from_str(&row.0).map_err(|e| KinforgeError::Storage(e.to_string()))?;
        let sex: Sex = row.1.parse().unwrap_or(Sex::Unknown);
        let names = self.get_names_for_person(&person_id)?;

        Ok(Person {
            id: person_id,
            names,
            sex,
            notes: row.2,
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
                    name_type: name_type_str.parse().unwrap_or(NameType::Birth),
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
                let pid =
                    PersonId::from_str(id).map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_person(&pid)
            })
            .collect()
    }

    /// Search people by name substring (case-insensitive, given or surname).
    pub fn search_people(&self, query: &str) -> KinforgeResult<Vec<Person>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT p.id FROM people p
                 JOIN person_names n ON n.person_id = p.id
                 WHERE LOWER(n.given) LIKE ?1 OR LOWER(n.surname) LIKE ?1
                 ORDER BY p.rowid",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map(params![pattern], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        ids.iter()
            .map(|id| {
                let pid =
                    PersonId::from_str(id).map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_person(&pid)
            })
            .collect()
    }

    pub fn person_count(&self) -> KinforgeResult<u64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM people", [], |r| r.get::<_, i64>(0))
            .map(|n| n as u64)
            .map_err(|e| KinforgeError::Storage(e.to_string()))
    }

    pub fn delete_person(&self, id: &PersonId) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM people WHERE id = ?1", params![id.as_str()])
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Person".to_string(),
                id: id.as_str(),
            });
        }
        Ok(())
    }

    /// Resolve a UUID prefix to a full Person.  Accepts any unambiguous leading
    /// substring of a UUID (e.g. `"abc123"` or `"abc123-45"`).
    pub fn find_person_by_id_prefix(&self, prefix: &str) -> KinforgeResult<Person> {
        let matches = self.ids_matching_prefix("people", prefix)?;
        self.resolve_prefix_match("Person", prefix, matches, |s| {
            let pid = PersonId::from_str(s).map_err(|e| KinforgeError::Storage(e.to_string()))?;
            self.get_person(&pid)
        })
    }

    /// Resolve a UUID prefix to a full Event.
    pub fn find_event_by_id_prefix(&self, prefix: &str) -> KinforgeResult<Event> {
        let matches = self.ids_matching_prefix("events", prefix)?;
        self.resolve_prefix_match("Event", prefix, matches, |s| {
            let eid = EventId::from_str(s).map_err(|e| KinforgeError::Storage(e.to_string()))?;
            self.get_event(&eid)
        })
    }

    /// Resolve a UUID prefix to a full Place.
    pub fn find_place_by_id_prefix(&self, prefix: &str) -> KinforgeResult<Place> {
        let matches = self.ids_matching_prefix("places", prefix)?;
        self.resolve_prefix_match("Place", prefix, matches, |s| {
            let pid = PlaceId::from_str(s).map_err(|e| KinforgeError::Storage(e.to_string()))?;
            self.get_place(&pid)
        })
    }

    /// Resolve a UUID prefix to a full Relationship.
    pub fn find_relationship_by_id_prefix(&self, prefix: &str) -> KinforgeResult<Relationship> {
        let matches = self.ids_matching_prefix("relationships", prefix)?;
        self.resolve_prefix_match("Relationship", prefix, matches, |s| {
            let rid =
                RelationshipId::from_str(s).map_err(|e| KinforgeError::Storage(e.to_string()))?;
            self.get_relationship(&rid)
        })
    }

    /// Resolve a UUID prefix to a full Source.
    pub fn find_source_by_id_prefix(&self, prefix: &str) -> KinforgeResult<Source> {
        let matches = self.ids_matching_prefix("sources", prefix)?;
        self.resolve_prefix_match("Source", prefix, matches, |s| {
            let sid = SourceId::from_str(s).map_err(|e| KinforgeError::Storage(e.to_string()))?;
            self.get_source(&sid)
        })
    }

    /// Resolve a UUID prefix to a full Citation.
    pub fn find_citation_by_id_prefix(&self, prefix: &str) -> KinforgeResult<Citation> {
        let matches = self.ids_matching_prefix("citations", prefix)?;
        self.resolve_prefix_match("Citation", prefix, matches, |s| {
            let cid =
                CitationId::from_str(s).map_err(|e| KinforgeError::Storage(e.to_string()))?;
            self.get_citation(&cid)
        })
    }

    // ── Internal prefix helpers ───────────────────────────────────────────────

    /// Return all IDs from `table` whose `id` column starts with `prefix`.
    /// Fetches at most 2 rows — enough to detect ambiguity without full scans.
    fn ids_matching_prefix(&self, table: &str, prefix: &str) -> KinforgeResult<Vec<String>> {
        // Table name is from our own code (not user input), safe to interpolate.
        let sql = format!("SELECT id FROM {} WHERE id LIKE ?1 LIMIT 2", table);
        let pattern = format!("{}%", prefix);
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        let ids: Vec<String> = stmt
            .query_map(params![pattern], |row| row.get(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(ids)
    }

    fn resolve_prefix_match<T, F>(
        &self,
        entity_type: &str,
        prefix: &str,
        matches: Vec<String>,
        fetch: F,
    ) -> KinforgeResult<T>
    where
        F: Fn(&str) -> KinforgeResult<T>,
    {
        match matches.len() {
            0 => Err(KinforgeError::NotFound {
                entity_type: entity_type.to_string(),
                id: prefix.to_string(),
            }),
            1 => fetch(&matches[0]),
            _ => Err(KinforgeError::InvalidField {
                field: "id".to_string(),
                value: format!(
                    "prefix '{}' is ambiguous — matches multiple {}s",
                    prefix, entity_type
                ),
            }),
        }
    }

    // ── Events ───────────────────────────────────────────────────────────────

    pub fn insert_event(&self, event: &Event) -> KinforgeResult<()> {
        let (date_kind, date_val, date_val2) = decompose_date(&event.date);
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

    pub fn update_event(&self, event: &Event) -> KinforgeResult<()> {
        let (date_kind, date_val, date_val2) = decompose_date(&event.date);
        let affected = self
            .conn
            .execute(
                "UPDATE events SET event_type = ?2, date_kind = ?3, date_value = ?4,
                 date_value2 = ?5, place_id = ?6, notes = ?7 WHERE id = ?1",
                params![
                    event.id.as_str(),
                    event.event_type.to_string(),
                    date_kind,
                    date_val,
                    date_val2,
                    event.place_id.as_ref().map(|p| p.as_str()),
                    event.notes,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Event".to_string(),
                id: event.id.as_str(),
            });
        }
        Ok(())
    }

    pub fn get_event(&self, id: &EventId) -> KinforgeResult<Event> {
        self.conn
            .query_row(
                "SELECT id, person_id, event_type, date_kind, date_value, date_value2, place_id, notes
                 FROM events WHERE id = ?1",
                params![id.as_str()],
                row_to_event,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => KinforgeError::NotFound {
                    entity_type: "Event".to_string(),
                    id: id.as_str(),
                },
                other => KinforgeError::Storage(other.to_string()),
            })
            .and_then(assemble_event)
    }

    pub fn list_events_for_person(&self, person_id: &PersonId) -> KinforgeResult<Vec<Event>> {
        fetch_event_rows(
            &self.conn,
            "SELECT id, person_id, event_type, date_kind, date_value, date_value2, place_id, notes
             FROM events WHERE person_id = ?1 ORDER BY rowid",
            Some(&person_id.as_str()),
        )
        .map_err(|e| KinforgeError::Storage(e.to_string()))?
        .into_iter()
        .map(assemble_event)
        .collect()
    }

    pub fn list_all_events(&self) -> KinforgeResult<Vec<Event>> {
        fetch_event_rows(
            &self.conn,
            "SELECT id, person_id, event_type, date_kind, date_value, date_value2, place_id, notes
             FROM events ORDER BY rowid",
            None,
        )
        .map_err(|e| KinforgeError::Storage(e.to_string()))?
        .into_iter()
        .map(assemble_event)
        .collect()
    }

    pub fn delete_event(&self, id: &EventId) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM events WHERE id = ?1", params![id.as_str()])
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Event".to_string(),
                id: id.as_str(),
            });
        }
        Ok(())
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

    pub fn update_place(&self, place: &Place) -> KinforgeResult<()> {
        let affected = self.conn
            .execute(
                "UPDATE places SET name = ?2, latitude = ?3, longitude = ?4, parent_id = ?5 WHERE id = ?1",
                params![
                    place.id.as_str(),
                    place.name,
                    place.latitude,
                    place.longitude,
                    place.parent_id.as_ref().map(|p| p.as_str()),
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Place".to_string(),
                id: place.id.as_str(),
            });
        }
        Ok(())
    }

    pub fn get_place(&self, id: &PlaceId) -> KinforgeResult<Place> {
        self.conn
            .query_row(
                "SELECT id, name, latitude, longitude, parent_id FROM places WHERE id = ?1",
                params![id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<f64>>(2)?,
                        row.get::<_, Option<f64>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
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
                Ok(Place {
                    id: pid,
                    name,
                    latitude: lat,
                    longitude: lon,
                    parent_id,
                })
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
                let pid =
                    PlaceId::from_str(id).map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_place(&pid)
            })
            .collect()
    }

    pub fn delete_place(&self, id: &PlaceId) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM places WHERE id = ?1", params![id.as_str()])
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Place".to_string(),
                id: id.as_str(),
            });
        }
        Ok(())
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

    pub fn update_relationship(&self, rel: &Relationship) -> KinforgeResult<()> {
        let affected = self.conn
            .execute(
                "UPDATE relationships SET rel_type = ?2, person1_id = ?3, person2_id = ?4, notes = ?5 WHERE id = ?1",
                params![
                    rel.id.as_str(),
                    rel.rel_type.to_string(),
                    rel.person1_id.as_str(),
                    rel.person2_id.as_str(),
                    rel.notes,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Relationship".to_string(),
                id: rel.id.as_str(),
            });
        }
        Ok(())
    }

    pub fn get_relationship(&self, id: &RelationshipId) -> KinforgeResult<Relationship> {
        self.conn
            .query_row(
                "SELECT id, rel_type, person1_id, person2_id, notes FROM relationships WHERE id = ?1",
                params![id.as_str()],
                |row| Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                )),
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
                let rel_type: RelationshipType = rt_str.parse()
                    .map_err(|e: KinforgeError| KinforgeError::Storage(e.to_string()))?;
                Ok(Relationship { id: rid, rel_type, person1_id: p1, person2_id: p2, notes })
            })
    }

    pub fn list_relationships_for_person(
        &self,
        person_id: &PersonId,
    ) -> KinforgeResult<Vec<Relationship>> {
        fetch_rel_rows(
            &self.conn,
            "SELECT id, rel_type, person1_id, person2_id, notes
             FROM relationships WHERE person1_id = ?1 OR person2_id = ?1 ORDER BY rowid",
            Some(&person_id.as_str()),
        )
        .map_err(|e| KinforgeError::Storage(e.to_string()))?
        .into_iter()
        .map(assemble_relationship)
        .collect()
    }

    pub fn list_all_relationships(&self) -> KinforgeResult<Vec<Relationship>> {
        fetch_rel_rows(
            &self.conn,
            "SELECT id, rel_type, person1_id, person2_id, notes FROM relationships ORDER BY rowid",
            None,
        )
        .map_err(|e| KinforgeError::Storage(e.to_string()))?
        .into_iter()
        .map(assemble_relationship)
        .collect()
    }

    pub fn delete_relationship(&self, id: &RelationshipId) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM relationships WHERE id = ?1",
                params![id.as_str()],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Relationship".to_string(),
                id: id.as_str(),
            });
        }
        Ok(())
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

    pub fn update_source(&self, source: &Source) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute(
                "UPDATE sources SET title = ?2, author = ?3, publication = ?4, year = ?5,
                 repository = ?6, notes = ?7 WHERE id = ?1",
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
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Source".to_string(),
                id: source.id.as_str(),
            });
        }
        Ok(())
    }

    pub fn get_source(&self, id: &SourceId) -> KinforgeResult<Source> {
        self.conn
            .query_row(
                "SELECT id, title, author, publication, year, repository, notes FROM sources WHERE id = ?1",
                params![id.as_str()],
                |row| Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i32>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                )),
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
                let sid =
                    SourceId::from_str(id).map_err(|e| KinforgeError::Storage(e.to_string()))?;
                self.get_source(&sid)
            })
            .collect()
    }

    pub fn delete_source(&self, id: &SourceId) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM sources WHERE id = ?1", params![id.as_str()])
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Source".to_string(),
                id: id.as_str(),
            });
        }
        Ok(())
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

    pub fn update_citation(&self, citation: &Citation) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute(
                "UPDATE citations SET page = ?2, confidence = ?3, notes = ?4 WHERE id = ?1",
                params![
                    citation.id.as_str(),
                    citation.page,
                    citation.confidence.to_string(),
                    citation.notes,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Citation".to_string(),
                id: citation.id.as_str(),
            });
        }
        Ok(())
    }

    pub fn get_citation(&self, id: &CitationId) -> KinforgeResult<Citation> {
        self.conn
            .query_row(
                "SELECT id, source_id, event_id, page, confidence, notes FROM citations WHERE id = ?1",
                params![id.as_str()],
                |row| Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                )),
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
                let confidence: ConfidenceLevel = conf_str.parse().unwrap_or(ConfidenceLevel::Secondary);
                Ok(Citation { id: cid, source_id: sid, event_id: eid, page, confidence, notes })
            })
    }

    pub fn list_citations_for_event(&self, event_id: &EventId) -> KinforgeResult<Vec<Citation>> {
        fetch_citation_rows(
            &self.conn,
            "SELECT id, source_id, event_id, page, confidence, notes
             FROM citations WHERE event_id = ?1 ORDER BY rowid",
            Some(&event_id.as_str()),
        )
        .map_err(|e| KinforgeError::Storage(e.to_string()))?
        .into_iter()
        .map(assemble_citation)
        .collect()
    }

    pub fn list_all_citations(&self) -> KinforgeResult<Vec<Citation>> {
        fetch_citation_rows(
            &self.conn,
            "SELECT id, source_id, event_id, page, confidence, notes FROM citations ORDER BY rowid",
            None,
        )
        .map_err(|e| KinforgeError::Storage(e.to_string()))?
        .into_iter()
        .map(assemble_citation)
        .collect()
    }

    pub fn delete_citation(&self, id: &CitationId) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM citations WHERE id = ?1", params![id.as_str()])
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Citation".to_string(),
                id: id.as_str(),
            });
        }
        Ok(())
    }

    /// List all citations that reference a given source.
    pub fn list_citations_for_source(&self, source_id: &SourceId) -> KinforgeResult<Vec<Citation>> {
        fetch_citation_rows(
            &self.conn,
            "SELECT id, source_id, event_id, page, confidence, notes
             FROM citations WHERE source_id = ?1 ORDER BY rowid",
            Some(&source_id.as_str()),
        )
        .map_err(|e| KinforgeError::Storage(e.to_string()))?
        .into_iter()
        .map(assemble_citation)
        .collect()
    }

    /// Reassign all events from `from` to `to` (for person merge).
    pub fn reassign_events_to_person(
        &self,
        from: &PersonId,
        to: &PersonId,
    ) -> KinforgeResult<usize> {
        self.conn
            .execute(
                "UPDATE events SET person_id = ?2 WHERE person_id = ?1",
                params![from.as_str(), to.as_str()],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))
    }

    /// Reassign all relationships involving `from` to use `to` instead.
    ///
    /// Handles both sides of each relationship.  After reassigning, self-loops
    /// (target ↔ target) and any rels that still point to `from` (duplicate
    /// check prevented the move) are deleted.
    pub fn reassign_relationships_to_person(
        &self,
        from: &PersonId,
        to: &PersonId,
    ) -> KinforgeResult<()> {
        // Remove any existing rels between from and to — they'd become self-loops.
        self.conn
            .execute(
                "DELETE FROM relationships
                 WHERE (person1_id = ?1 AND person2_id = ?2)
                    OR (person1_id = ?2 AND person2_id = ?1)",
                params![from.as_str(), to.as_str()],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        // Reassign person1_id references.
        self.conn
            .execute(
                "UPDATE relationships SET person1_id = ?2 WHERE person1_id = ?1",
                params![from.as_str(), to.as_str()],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        // Reassign person2_id references.
        self.conn
            .execute(
                "UPDATE relationships SET person2_id = ?2 WHERE person2_id = ?1",
                params![from.as_str(), to.as_str()],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        // Delete any accidental self-loops.
        self.conn
            .execute(
                "DELETE FROM relationships WHERE person1_id = person2_id",
                [],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        Ok(())
    }

    // ── Statistics ────────────────────────────────────────────────────────────

    pub fn stats(&self) -> KinforgeResult<DatabaseStats> {
        let people = self
            .conn
            .query_row("SELECT COUNT(*) FROM people", [], |r| r.get::<_, i64>(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))? as u64;
        let events = self
            .conn
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get::<_, i64>(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))? as u64;
        let sources = self
            .conn
            .query_row("SELECT COUNT(*) FROM sources", [], |r| r.get::<_, i64>(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))? as u64;
        let citations = self
            .conn
            .query_row("SELECT COUNT(*) FROM citations", [], |r| r.get::<_, i64>(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))? as u64;
        let relationships = self
            .conn
            .query_row("SELECT COUNT(*) FROM relationships", [], |r| {
                r.get::<_, i64>(0)
            })
            .map_err(|e| KinforgeError::Storage(e.to_string()))? as u64;
        let places = self
            .conn
            .query_row("SELECT COUNT(*) FROM places", [], |r| r.get::<_, i64>(0))
            .map_err(|e| KinforgeError::Storage(e.to_string()))? as u64;

        Ok(DatabaseStats {
            people,
            events,
            sources,
            citations,
            relationships,
            places,
        })
    }

    // ── Full-text search (FTS5) ──────────────────────────────────────────────

    /// Rebuild the FTS5 index from all current data. Fast for typical databases.
    pub fn rebuild_fts_index(&self) -> KinforgeResult<()> {
        self.conn
            .execute("DELETE FROM fts_index", [])
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        // Person names
        self.conn
            .execute_batch(
                "INSERT INTO fts_index(body, entity_type, entity_id)
                 SELECT TRIM(COALESCE(given,'') || ' ' || COALESCE(surname,'')),
                        'person', person_id
                 FROM person_names
                 WHERE given IS NOT NULL OR surname IS NOT NULL",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        // Person notes
        self.conn
            .execute_batch(
                "INSERT INTO fts_index(body, entity_type, entity_id)
                 SELECT notes, 'person', id FROM people WHERE notes IS NOT NULL",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        // Event notes
        self.conn
            .execute_batch(
                "INSERT INTO fts_index(body, entity_type, entity_id)
                 SELECT notes, 'event', id FROM events WHERE notes IS NOT NULL",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        // Source titles + authors
        self.conn
            .execute_batch(
                "INSERT INTO fts_index(body, entity_type, entity_id)
                 SELECT TRIM(title || ' ' || COALESCE(author,'')), 'source', id FROM sources",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        // Source notes
        self.conn
            .execute_batch(
                "INSERT INTO fts_index(body, entity_type, entity_id)
                 SELECT notes, 'source', id FROM sources WHERE notes IS NOT NULL",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        // Place names
        self.conn
            .execute_batch(
                "INSERT INTO fts_index(body, entity_type, entity_id)
                 SELECT name, 'place', id FROM places",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        Ok(())
    }

    /// Search all indexed text using FTS5. Rebuilds index before querying.
    /// Returns results ordered by relevance (best match first).
    pub fn search_fulltext(&self, query: &str) -> KinforgeResult<Vec<FtsResult>> {
        self.rebuild_fts_index()?;

        let mut stmt = self
            .conn
            .prepare(
                "SELECT entity_type, entity_id, body
                 FROM fts_index
                 WHERE fts_index MATCH ?1
                 ORDER BY rank
                 LIMIT 100",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![query], |row| {
                Ok(FtsResult {
                    entity_type: row.get(0)?,
                    entity_id: row.get(1)?,
                    snippet: row.get(2)?,
                })
            })
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;

        rows.map(|r| r.map_err(|e| KinforgeError::Storage(e.to_string())))
            .collect()
    }

    // ── Media ────────────────────────────────────────────────────────────────

    pub fn insert_media(&self, media: &Media) -> KinforgeResult<()> {
        self.conn
            .execute(
                "INSERT INTO media (id, title, path, url, media_type, description, date)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    media.id.as_str(),
                    media.title,
                    media.path,
                    media.url,
                    media.media_type.to_string(),
                    media.description,
                    media.date,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn update_media(&self, media: &Media) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute(
                "UPDATE media SET title=?2, path=?3, url=?4, media_type=?5, description=?6, date=?7
                 WHERE id=?1",
                params![
                    media.id.as_str(),
                    media.title,
                    media.path,
                    media.url,
                    media.media_type.to_string(),
                    media.description,
                    media.date,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Media".to_string(),
                id: media.id.as_str(),
            });
        }
        Ok(())
    }

    pub fn delete_media(&self, id: &MediaId) -> KinforgeResult<()> {
        self.conn
            .execute("DELETE FROM media WHERE id = ?1", params![id.as_str()])
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn get_media(&self, id: &MediaId) -> KinforgeResult<Media> {
        self.conn
            .query_row(
                "SELECT id, title, path, url, media_type, description, date FROM media WHERE id=?1",
                params![id.as_str()],
                Self::row_to_media,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => KinforgeError::NotFound {
                    entity_type: "Media".to_string(),
                    id: id.as_str(),
                },
                other => KinforgeError::Storage(other.to_string()),
            })
    }

    pub fn list_media(&self) -> KinforgeResult<Vec<Media>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, title, path, url, media_type, description, date FROM media ORDER BY title",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], Self::row_to_media)
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        rows.map(|r| r.map_err(|e| KinforgeError::Storage(e.to_string())))
            .collect()
    }

    fn row_to_media(row: &rusqlite::Row<'_>) -> rusqlite::Result<Media> {
        let media_type_str: String = row.get(4)?;
        let media_type = media_type_str.parse::<MediaType>().unwrap_or(MediaType::Other);
        Ok(Media {
            id: MediaId::from_str(&row.get::<_, String>(0)?).unwrap_or_default(),
            title: row.get(1)?,
            path: row.get(2)?,
            url: row.get(3)?,
            media_type,
            description: row.get(5)?,
            date: row.get(6)?,
        })
    }

    // ── Media Links ──────────────────────────────────────────────────────────

    pub fn insert_media_link(&self, link: &MediaLink) -> KinforgeResult<()> {
        self.conn
            .execute(
                "INSERT INTO media_links (id, media_id, entity_type, entity_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    link.id.as_str(),
                    link.media_id.as_str(),
                    link.entity_type.to_string(),
                    link.entity_id,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn delete_media_link(&self, id: &MediaLinkId) -> KinforgeResult<()> {
        self.conn
            .execute("DELETE FROM media_links WHERE id=?1", params![id.as_str()])
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn list_media_links_for_media(&self, media_id: &MediaId) -> KinforgeResult<Vec<MediaLink>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, media_id, entity_type, entity_id
                 FROM media_links WHERE media_id=?1",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![media_id.as_str()], Self::row_to_media_link)
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        rows.map(|r| r.map_err(|e| KinforgeError::Storage(e.to_string())))
            .collect()
    }

    pub fn list_media_for_entity(
        &self,
        entity_type: &MediaEntityType,
        entity_id: &str,
    ) -> KinforgeResult<Vec<Media>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT m.id, m.title, m.path, m.url, m.media_type, m.description, m.date
                 FROM media m
                 JOIN media_links ml ON ml.media_id = m.id
                 WHERE ml.entity_type = ?1 AND ml.entity_id = ?2
                 ORDER BY m.title",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(
                params![entity_type.to_string(), entity_id],
                Self::row_to_media,
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        rows.map(|r| r.map_err(|e| KinforgeError::Storage(e.to_string())))
            .collect()
    }

    fn row_to_media_link(row: &rusqlite::Row<'_>) -> rusqlite::Result<MediaLink> {
        let entity_type_str: String = row.get(2)?;
        let entity_type = entity_type_str
            .parse::<MediaEntityType>()
            .unwrap_or(MediaEntityType::Person);
        Ok(MediaLink {
            id: MediaLinkId::from_str(&row.get::<_, String>(0)?).unwrap_or_default(),
            media_id: MediaId::from_str(&row.get::<_, String>(1)?).unwrap_or_default(),
            entity_type,
            entity_id: row.get(3)?,
        })
    }

    // ── Research Tasks ───────────────────────────────────────────────────────

    pub fn insert_task(&self, task: &Task) -> KinforgeResult<()> {
        self.conn
            .execute(
                "INSERT INTO research_tasks
                 (id, description, person_id, priority, status, notes, created, updated)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    task.id.as_str(),
                    task.description,
                    task.person_id.as_ref().map(|p| p.as_str()),
                    task.priority.to_string(),
                    task.status.to_string(),
                    task.notes,
                    task.created,
                    task.updated,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn update_task(&self, task: &Task) -> KinforgeResult<()> {
        let affected = self
            .conn
            .execute(
                "UPDATE research_tasks
                 SET description=?2, person_id=?3, priority=?4, status=?5,
                     notes=?6, updated=?7
                 WHERE id=?1",
                params![
                    task.id.as_str(),
                    task.description,
                    task.person_id.as_ref().map(|p| p.as_str()),
                    task.priority.to_string(),
                    task.status.to_string(),
                    task.notes,
                    task.updated,
                ],
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(KinforgeError::NotFound {
                entity_type: "Task".to_string(),
                id: task.id.as_str(),
            });
        }
        Ok(())
    }

    pub fn delete_task(&self, id: &TaskId) -> KinforgeResult<()> {
        self.conn
            .execute("DELETE FROM research_tasks WHERE id=?1", params![id.as_str()])
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn get_task(&self, id: &TaskId) -> KinforgeResult<Task> {
        self.conn
            .query_row(
                "SELECT id,description,person_id,priority,status,notes,created,updated
                 FROM research_tasks WHERE id=?1",
                params![id.as_str()],
                Self::row_to_task,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => KinforgeError::NotFound {
                    entity_type: "Task".to_string(),
                    id: id.as_str(),
                },
                other => KinforgeError::Storage(other.to_string()),
            })
    }

    pub fn list_tasks(&self) -> KinforgeResult<Vec<Task>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,description,person_id,priority,status,notes,created,updated
                 FROM research_tasks
                 ORDER BY CASE status WHEN 'InProgress' THEN 0 WHEN 'Pending' THEN 1 ELSE 2 END,
                          CASE priority WHEN 'High' THEN 0 WHEN 'Medium' THEN 1 ELSE 2 END,
                          created",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], Self::row_to_task)
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        rows.map(|r| r.map_err(|e| KinforgeError::Storage(e.to_string())))
            .collect()
    }

    pub fn list_tasks_for_person(&self, person_id: &PersonId) -> KinforgeResult<Vec<Task>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,description,person_id,priority,status,notes,created,updated
                 FROM research_tasks WHERE person_id=?1
                 ORDER BY created",
            )
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![person_id.as_str()], Self::row_to_task)
            .map_err(|e| KinforgeError::Storage(e.to_string()))?;
        rows.map(|r| r.map_err(|e| KinforgeError::Storage(e.to_string())))
            .collect()
    }

    fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
        let priority: TaskPriority = row
            .get::<_, String>(3)?
            .parse()
            .unwrap_or(TaskPriority::Medium);
        let status: TaskStatus = row
            .get::<_, String>(4)?
            .parse()
            .unwrap_or(TaskStatus::Pending);
        let person_id_str: Option<String> = row.get(2)?;
        let person_id = person_id_str
            .as_deref()
            .and_then(|s| PersonId::from_str(s).ok());
        Ok(Task {
            id: TaskId::from_str(&row.get::<_, String>(0)?).unwrap_or_default(),
            description: row.get(1)?,
            person_id,
            priority,
            status,
            notes: row.get(5)?,
            created: row.get(6)?,
            updated: row.get(7)?,
        })
    }
}

// ── Row helpers ───────────────────────────────────────────────────────────────

type EventRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

// ── Standalone query helpers (stmt fully scoped inside each fn) ───────────────

fn fetch_event_rows(
    conn: &Connection,
    sql: &str,
    param: Option<&str>,
) -> rusqlite::Result<Vec<EventRow>> {
    let mut stmt = conn.prepare(sql)?;
    match param {
        Some(p) => stmt.query_map([p], row_to_event)?.collect(),
        None => stmt.query_map([], row_to_event)?.collect(),
    }
}

fn fetch_rel_rows(
    conn: &Connection,
    sql: &str,
    param: Option<&str>,
) -> rusqlite::Result<Vec<RelRow>> {
    let mut stmt = conn.prepare(sql)?;
    match param {
        Some(p) => stmt
            .query_map([p], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?
            .collect(),
        None => stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?
            .collect(),
    }
}

fn fetch_citation_rows(
    conn: &Connection,
    sql: &str,
    param: Option<&str>,
) -> rusqlite::Result<Vec<CitationRow>> {
    let mut stmt = conn.prepare(sql)?;
    match param {
        Some(p) => stmt
            .query_map([p], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?
            .collect(),
        None => stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?
            .collect(),
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<EventRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
    ))
}

fn assemble_event(
    (id_str, pid_str, et_str, dk, dv, dv2, place_str, notes): EventRow,
) -> KinforgeResult<Event> {
    let eid = EventId::from_str(&id_str).map_err(|e| KinforgeError::Storage(e.to_string()))?;
    let pid = PersonId::from_str(&pid_str).map_err(|e| KinforgeError::Storage(e.to_string()))?;
    let event_type: EventType = et_str.parse().unwrap_or(EventType::Other(et_str));
    let date = dk
        .as_deref()
        .and_then(|k| EventDate::from_parts(k, dv.as_deref(), dv2.as_deref()));
    let place_id = place_str.as_deref().and_then(|s| PlaceId::from_str(s).ok());
    Ok(Event {
        id: eid,
        person_id: pid,
        event_type,
        date,
        place_id,
        notes,
    })
}

type CitationRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
);

fn assemble_citation(
    (id_str, sid_str, eid_str, page, conf_str, notes): CitationRow,
) -> KinforgeResult<Citation> {
    let cid = CitationId::from_str(&id_str).map_err(|e| KinforgeError::Storage(e.to_string()))?;
    let sid = SourceId::from_str(&sid_str).map_err(|e| KinforgeError::Storage(e.to_string()))?;
    let eid = EventId::from_str(&eid_str).map_err(|e| KinforgeError::Storage(e.to_string()))?;
    let confidence: ConfidenceLevel = conf_str.parse().unwrap_or(ConfidenceLevel::Secondary);
    Ok(Citation {
        id: cid,
        source_id: sid,
        event_id: eid,
        page,
        confidence,
        notes,
    })
}

fn assemble_relationship(
    (id_str, rt_str, p1_str, p2_str, notes): (String, String, String, String, Option<String>),
) -> KinforgeResult<Relationship> {
    let rid =
        RelationshipId::from_str(&id_str).map_err(|e| KinforgeError::Storage(e.to_string()))?;
    let p1 = PersonId::from_str(&p1_str).map_err(|e| KinforgeError::Storage(e.to_string()))?;
    let p2 = PersonId::from_str(&p2_str).map_err(|e| KinforgeError::Storage(e.to_string()))?;
    let rel_type: RelationshipType = rt_str
        .parse()
        .map_err(|e: KinforgeError| KinforgeError::Storage(e.to_string()))?;
    Ok(Relationship {
        id: rid,
        rel_type,
        person1_id: p1,
        person2_id: p2,
        notes,
    })
}

fn decompose_date(date: &Option<EventDate>) -> (Option<String>, Option<String>, Option<String>) {
    match date {
        Some(d) => (Some(d.kind_str().to_string()), d.date_str(), d.date2_str()),
        None => (None, None, None),
    }
}

// ── Stats ──────────────────────────────────────────────────────────────────────

pub struct DatabaseStats {
    pub people: u64,
    pub events: u64,
    pub sources: u64,
    pub citations: u64,
    pub relationships: u64,
    pub places: u64,
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn make_person(given: &str, surname: &str, sex: Sex) -> Person {
        let mut p = Person::new(sex);
        p.names.push(PersonName {
            given: Some(given.to_string()),
            surname: Some(surname.to_string()),
            name_type: NameType::Birth,
            prefix: None,
            suffix: None,
        });
        p
    }

    #[test]
    fn test_person_roundtrip() {
        let db = make_db();
        let person = make_person("John", "Smith", Sex::Male);
        db.insert_person(&person).unwrap();
        let fetched = db.get_person(&person.id).unwrap();
        assert_eq!(fetched.id, person.id);
        assert_eq!(fetched.sex, Sex::Male);
        assert_eq!(fetched.names[0].given, Some("John".to_string()));
        assert_eq!(fetched.names[0].surname, Some("Smith".to_string()));
    }

    #[test]
    fn test_person_update() {
        let db = make_db();
        let mut person = make_person("Jane", "Doe", Sex::Female);
        db.insert_person(&person).unwrap();

        person.notes = Some("Updated notes".to_string());
        person.names[0].given = Some("Janet".to_string());
        db.update_person(&person).unwrap();

        let fetched = db.get_person(&person.id).unwrap();
        assert_eq!(fetched.notes, Some("Updated notes".to_string()));
        assert_eq!(fetched.names[0].given, Some("Janet".to_string()));
    }

    #[test]
    fn test_person_delete() {
        let db = make_db();
        let person = make_person("Tom", "Jones", Sex::Male);
        db.insert_person(&person).unwrap();
        db.delete_person(&person.id).unwrap();
        assert!(db.get_person(&person.id).is_err());
    }

    #[test]
    fn test_person_delete_cascades_events() {
        let db = make_db();
        let person = make_person("Tom", "Jones", Sex::Male);
        db.insert_person(&person).unwrap();
        let event = Event::new(EventType::Birth, person.id.clone());
        db.insert_event(&event).unwrap();
        db.delete_person(&person.id).unwrap();
        assert!(db.get_event(&event.id).is_err());
    }

    #[test]
    fn test_search_people() {
        let db = make_db();
        db.insert_person(&make_person("Alice", "Anderson", Sex::Female))
            .unwrap();
        db.insert_person(&make_person("Bob", "Baker", Sex::Male))
            .unwrap();
        db.insert_person(&make_person("Alice", "Baker", Sex::Female))
            .unwrap();

        let results = db.search_people("Alice").unwrap();
        assert_eq!(results.len(), 2);

        let results = db.search_people("Baker").unwrap();
        assert_eq!(results.len(), 2);

        let results = db.search_people("Anderson").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_event_roundtrip() {
        let db = make_db();
        let person = make_person("Eva", "Green", Sex::Female);
        db.insert_person(&person).unwrap();

        let mut event = Event::new(EventType::Birth, person.id.clone());
        event.notes = Some("Hospital birth".to_string());
        db.insert_event(&event).unwrap();

        let fetched = db.get_event(&event.id).unwrap();
        assert_eq!(fetched.event_type, EventType::Birth);
        assert_eq!(fetched.notes, Some("Hospital birth".to_string()));
    }

    #[test]
    fn test_event_with_date() {
        use chrono::NaiveDate;
        let db = make_db();
        let person = make_person("Bob", "Hope", Sex::Male);
        db.insert_person(&person).unwrap();

        let d = NaiveDate::from_ymd_opt(1903, 5, 29).unwrap();
        let mut event = Event::new(EventType::Birth, person.id.clone());
        event.date = Some(EventDate::Exact(d));
        db.insert_event(&event).unwrap();

        let fetched = db.get_event(&event.id).unwrap();
        match fetched.date {
            Some(EventDate::Exact(fd)) => assert_eq!(fd, d),
            _ => panic!("Expected Exact date"),
        }
    }

    #[test]
    fn test_event_delete() {
        let db = make_db();
        let person = make_person("Test", "Person", Sex::Unknown);
        db.insert_person(&person).unwrap();
        let event = Event::new(EventType::Death, person.id.clone());
        db.insert_event(&event).unwrap();
        db.delete_event(&event.id).unwrap();
        assert!(db.get_event(&event.id).is_err());
    }

    #[test]
    fn test_relationship_roundtrip() {
        let db = make_db();
        let parent = make_person("Father", "Smith", Sex::Male);
        let child = make_person("Child", "Smith", Sex::Female);
        db.insert_person(&parent).unwrap();
        db.insert_person(&child).unwrap();

        let rel = Relationship::new(
            RelationshipType::ParentChild,
            parent.id.clone(),
            child.id.clone(),
        );
        db.insert_relationship(&rel).unwrap();

        let fetched = db.get_relationship(&rel.id).unwrap();
        assert_eq!(fetched.rel_type, RelationshipType::ParentChild);
        assert_eq!(fetched.person1_id, parent.id);
        assert_eq!(fetched.person2_id, child.id);
    }

    #[test]
    fn test_relationship_delete() {
        let db = make_db();
        let p1 = make_person("A", "X", Sex::Male);
        let p2 = make_person("B", "X", Sex::Female);
        db.insert_person(&p1).unwrap();
        db.insert_person(&p2).unwrap();
        let rel = Relationship::new(RelationshipType::Spouse, p1.id.clone(), p2.id.clone());
        db.insert_relationship(&rel).unwrap();
        db.delete_relationship(&rel.id).unwrap();
        assert!(db.get_relationship(&rel.id).is_err());
    }

    #[test]
    fn test_source_roundtrip() {
        let db = make_db();
        let mut source = Source::new("1900 US Census");
        source.author = Some("US Bureau of the Census".to_string());
        source.year = Some(1900);
        db.insert_source(&source).unwrap();

        let fetched = db.get_source(&source.id).unwrap();
        assert_eq!(fetched.title, "1900 US Census");
        assert_eq!(fetched.year, Some(1900));
    }

    #[test]
    fn test_source_update_and_delete() {
        let db = make_db();
        let mut source = Source::new("Draft Title");
        db.insert_source(&source).unwrap();
        source.title = "Final Title".to_string();
        db.update_source(&source).unwrap();
        let fetched = db.get_source(&source.id).unwrap();
        assert_eq!(fetched.title, "Final Title");
        db.delete_source(&source.id).unwrap();
        assert!(db.get_source(&source.id).is_err());
    }

    #[test]
    fn test_citation_roundtrip() {
        let db = make_db();
        let person = make_person("Ann", "Lee", Sex::Female);
        db.insert_person(&person).unwrap();
        let event = Event::new(EventType::Birth, person.id.clone());
        db.insert_event(&event).unwrap();
        let source = Source::new("Vital Records");
        db.insert_source(&source).unwrap();

        let mut cit = Citation::new(source.id.clone(), event.id.clone());
        cit.page = Some("p.42".to_string());
        cit.confidence = ConfidenceLevel::Primary;
        db.insert_citation(&cit).unwrap();

        let fetched = db.get_citation(&cit.id).unwrap();
        assert_eq!(fetched.page, Some("p.42".to_string()));
        assert_eq!(fetched.confidence, ConfidenceLevel::Primary);
    }

    #[test]
    fn test_stats() {
        let db = make_db();
        let p = make_person("Stan", "Lee", Sex::Male);
        db.insert_person(&p).unwrap();
        let e = Event::new(EventType::Birth, p.id.clone());
        db.insert_event(&e).unwrap();

        let stats = db.stats().unwrap();
        assert_eq!(stats.people, 1);
        assert_eq!(stats.events, 1);
    }

    #[test]
    fn test_list_people_multiple() {
        let db = make_db();
        for i in 0..5 {
            db.insert_person(&make_person(&format!("Person{}", i), "Test", Sex::Unknown))
                .unwrap();
        }
        assert_eq!(db.list_people().unwrap().len(), 5);
    }
}
