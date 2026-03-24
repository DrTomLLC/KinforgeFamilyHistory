use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;

// ── Person query ──────────────────────────────────────────────────────────────

/// Fluent filter for querying people.
#[derive(Default)]
pub struct PersonQuery {
    pub name_contains: Option<String>,
    pub given_contains: Option<String>,
    pub surname_contains: Option<String>,
    pub sex: Option<Sex>,
}

impl PersonQuery {
    pub fn new() -> Self {
        Self::default()
    }

    /// Match people whose given or surname contains `s`.
    pub fn name_contains(mut self, s: impl Into<String>) -> Self {
        self.name_contains = Some(s.into().to_lowercase());
        self
    }

    pub fn given_contains(mut self, s: impl Into<String>) -> Self {
        self.given_contains = Some(s.into().to_lowercase());
        self
    }

    pub fn surname_contains(mut self, s: impl Into<String>) -> Self {
        self.surname_contains = Some(s.into().to_lowercase());
        self
    }

    pub fn sex(mut self, sex: Sex) -> Self {
        self.sex = Some(sex);
        self
    }

    pub fn run(&self, db: &Database) -> KinforgeResult<Vec<Person>> {
        // If we have a name filter, use storage-level SQL search for efficiency
        let candidates = if let Some(ref q) = self.name_contains {
            db.search_people(q)?
        } else {
            db.list_people()?
        };

        Ok(candidates
            .into_iter()
            .filter(|p| self.matches(p))
            .collect())
    }

    fn matches(&self, person: &Person) -> bool {
        if let Some(ref s) = self.sex {
            if &person.sex != s {
                return false;
            }
        }
        if let Some(ref f) = self.surname_contains {
            let found = person.names.iter().any(|n| {
                n.surname.as_deref().map(|s| s.to_lowercase().contains(f.as_str())).unwrap_or(false)
            });
            if !found {
                return false;
            }
        }
        if let Some(ref f) = self.given_contains {
            let found = person.names.iter().any(|n| {
                n.given.as_deref().map(|s| s.to_lowercase().contains(f.as_str())).unwrap_or(false)
            });
            if !found {
                return false;
            }
        }
        true
    }
}

// ── Event query ───────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct EventQuery {
    pub person_id: Option<PersonId>,
    pub event_type: Option<EventType>,
    pub place_name_contains: Option<String>,
}

impl EventQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_person(mut self, id: PersonId) -> Self {
        self.person_id = Some(id);
        self
    }

    pub fn of_type(mut self, et: EventType) -> Self {
        self.event_type = Some(et);
        self
    }

    pub fn place_contains(mut self, s: impl Into<String>) -> Self {
        self.place_name_contains = Some(s.into().to_lowercase());
        self
    }

    pub fn run(&self, db: &Database) -> KinforgeResult<Vec<Event>> {
        let events = if let Some(ref pid) = self.person_id {
            db.list_events_for_person(pid)?
        } else {
            db.list_all_events()?
        };

        let mut result: Vec<Event> = events
            .into_iter()
            .filter(|e| {
                if let Some(ref et) = self.event_type {
                    if std::mem::discriminant(&e.event_type) != std::mem::discriminant(et) {
                        return false;
                    }
                }
                true
            })
            .collect();

        if let Some(ref place_filter) = self.place_name_contains {
            // Filter by place name (requires extra DB lookups, done in-memory)
            result.retain(|e| {
                e.place_id.as_ref().map_or(false, |pid| {
                    db.get_place(pid)
                        .map(|pl| pl.name.to_lowercase().contains(place_filter.as_str()))
                        .unwrap_or(false)
                })
            });
        }

        Ok(result)
    }
}

// ── Source query ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct SourceQuery {
    pub title_contains: Option<String>,
    pub author_contains: Option<String>,
    pub year_range: Option<(i32, i32)>,
}

impl SourceQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title_contains(mut self, s: impl Into<String>) -> Self {
        self.title_contains = Some(s.into().to_lowercase());
        self
    }

    pub fn author_contains(mut self, s: impl Into<String>) -> Self {
        self.author_contains = Some(s.into().to_lowercase());
        self
    }

    pub fn year_range(mut self, from: i32, to: i32) -> Self {
        self.year_range = Some((from, to));
        self
    }

    pub fn run(&self, db: &Database) -> KinforgeResult<Vec<Source>> {
        let all = db.list_sources()?;
        Ok(all
            .into_iter()
            .filter(|s| {
                if let Some(ref f) = self.title_contains {
                    if !s.title.to_lowercase().contains(f.as_str()) {
                        return false;
                    }
                }
                if let Some(ref f) = self.author_contains {
                    let matches = s.author.as_deref()
                        .map(|a| a.to_lowercase().contains(f.as_str()))
                        .unwrap_or(false);
                    if !matches {
                        return false;
                    }
                }
                if let Some((from, to)) = self.year_range {
                    match s.year {
                        Some(y) if y >= from && y <= to => {}
                        _ => return false,
                    }
                }
                true
            })
            .collect())
    }
}

// ── Convenience function ──────────────────────────────────────────────────────

/// Quick full-text search across people names.
pub fn search_people(db: &Database, query: &str) -> KinforgeResult<Vec<Person>> {
    db.search_people(query)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kinforge_storage::Database;

    fn setup() -> Database {
        let db = Database::open_in_memory().unwrap();
        let names = [("Alice", "Anderson", Sex::Female), ("Bob", "Baker", Sex::Male),
                     ("Carol", "Anderson", Sex::Female)];
        for (g, s, sex) in names {
            let mut p = Person::new(sex);
            p.names.push(PersonName {
                given: Some(g.to_string()), surname: Some(s.to_string()),
                name_type: NameType::Birth, prefix: None, suffix: None,
            });
            db.insert_person(&p).unwrap();
        }
        db
    }

    #[test]
    fn test_person_query_surname() {
        let db = setup();
        let results = PersonQuery::new().surname_contains("Anderson").run(&db).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_person_query_sex() {
        let db = setup();
        let results = PersonQuery::new().sex(Sex::Male).run(&db).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].names[0].given, Some("Bob".to_string()));
    }

    #[test]
    fn test_search_people() {
        let db = setup();
        let results = search_people(&db, "ali").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_source_query() {
        let db = Database::open_in_memory().unwrap();
        let mut s1 = Source::new("1900 US Census"); s1.year = Some(1900);
        let mut s2 = Source::new("1910 US Census"); s2.year = Some(1910);
        let mut s3 = Source::new("Birth Register"); s3.year = Some(1850);
        db.insert_source(&s1).unwrap();
        db.insert_source(&s2).unwrap();
        db.insert_source(&s3).unwrap();

        let results = SourceQuery::new().title_contains("census").run(&db).unwrap();
        assert_eq!(results.len(), 2);

        let results = SourceQuery::new().year_range(1900, 1910).run(&db).unwrap();
        assert_eq!(results.len(), 2);
    }
}
