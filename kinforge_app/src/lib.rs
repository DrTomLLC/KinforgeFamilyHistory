use kinforge_config::Config;
use kinforge_core::{models::*, validation, KinforgeError, KinforgeResult};
use kinforge_storage::{repository::DatabaseStats, Database};
use std::path::Path;

pub struct Application {
    pub db: Database,
    pub config: Config,
}

impl Application {
    pub fn open(config: Config) -> KinforgeResult<Self> {
        // Ensure the database parent directory exists.
        if let Some(parent) = config.database_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(KinforgeError::Io)?;
            }
        }

        // Backup if configured.
        if config.backup_on_open && config.database_path.exists() {
            if let Err(e) = create_backup(&config) {
                eprintln!("Warning: backup failed: {}", e);
            }
        }

        let db = Database::open(&config.database_path)?;
        Ok(Self { db, config })
    }

    pub fn open_in_memory() -> KinforgeResult<Self> {
        let db = Database::open_in_memory()?;
        Ok(Self {
            db,
            config: Config::default(),
        })
    }

    // ── Statistics ─────────────────────────────────────────────────────────

    pub fn stats(&self) -> KinforgeResult<DatabaseStats> {
        self.db.stats()
    }

    // ── People ─────────────────────────────────────────────────────────────

    pub fn add_person(
        &self,
        given: Option<&str>,
        surname: Option<&str>,
        sex: Sex,
        notes: Option<&str>,
    ) -> KinforgeResult<Person> {
        let mut person = Person::new(sex);
        if given.is_some() || surname.is_some() {
            person.names.push(PersonName {
                given: given.map(|s| s.to_string()),
                surname: surname.map(|s| s.to_string()),
                name_type: NameType::Birth,
                prefix: None,
                suffix: None,
            });
        }
        person.notes = notes.map(|s| s.to_string());
        validation::validate_person(&person)?;
        self.db.insert_person(&person)?;
        Ok(person)
    }

    /// Add an additional name to an existing person.
    pub fn add_name_to_person(
        &self,
        person_id: &PersonId,
        given: Option<&str>,
        surname: Option<&str>,
        name_type: NameType,
    ) -> KinforgeResult<Person> {
        let mut person = self.db.get_person(person_id)?;
        person.names.push(PersonName {
            given: given.map(|s| s.to_string()),
            surname: surname.map(|s| s.to_string()),
            name_type,
            prefix: None,
            suffix: None,
        });
        validation::validate_person(&person)?;
        self.db.update_person(&person)?;
        Ok(person)
    }

    pub fn update_person(&self, person: Person) -> KinforgeResult<Person> {
        validation::validate_person(&person)?;
        self.db.update_person(&person)?;
        Ok(person)
    }

    pub fn get_person(&self, id: &PersonId) -> KinforgeResult<Person> {
        self.db.get_person(id)
    }

    pub fn list_people(&self) -> KinforgeResult<Vec<Person>> {
        self.db.list_people()
    }

    pub fn delete_person(&self, id: &PersonId) -> KinforgeResult<()> {
        self.db.delete_person(id)
    }

    // ── Events ─────────────────────────────────────────────────────────────

    pub fn add_event(
        &self,
        person_id: PersonId,
        event_type: EventType,
        date: Option<EventDate>,
        place_name: Option<&str>,
        notes: Option<&str>,
    ) -> KinforgeResult<Event> {
        let place_id = if let Some(name) = place_name {
            let place = Place::new(name);
            validation::validate_place(&place)?;
            self.db.insert_place(&place)?;
            Some(place.id)
        } else {
            None
        };

        let mut event = Event::new(event_type, person_id);
        event.date = date;
        event.place_id = place_id;
        event.notes = notes.map(|s| s.to_string());
        validation::validate_event(&event)?;
        self.db.insert_event(&event)?;
        Ok(event)
    }

    pub fn update_event(&self, event: Event) -> KinforgeResult<Event> {
        validation::validate_event(&event)?;
        self.db.update_event(&event)?;
        Ok(event)
    }

    pub fn delete_event(&self, id: &EventId) -> KinforgeResult<()> {
        self.db.delete_event(id)
    }

    pub fn list_events_for_person(&self, person_id: &PersonId) -> KinforgeResult<Vec<Event>> {
        self.db.list_events_for_person(person_id)
    }

    pub fn get_event(&self, id: &EventId) -> KinforgeResult<Event> {
        self.db.get_event(id)
    }

    // ── Places ─────────────────────────────────────────────────────────────

    pub fn add_place(
        &self,
        name: &str,
        latitude: Option<f64>,
        longitude: Option<f64>,
    ) -> KinforgeResult<Place> {
        let mut place = Place::new(name);
        place.latitude = latitude;
        place.longitude = longitude;
        validation::validate_place(&place)?;
        self.db.insert_place(&place)?;
        Ok(place)
    }

    pub fn update_place(&self, place: Place) -> KinforgeResult<Place> {
        validation::validate_place(&place)?;
        self.db.update_place(&place)?;
        Ok(place)
    }

    pub fn delete_place(&self, id: &PlaceId) -> KinforgeResult<()> {
        self.db.delete_place(id)
    }

    pub fn list_places(&self) -> KinforgeResult<Vec<Place>> {
        self.db.list_places()
    }

    // ── Relationships ──────────────────────────────────────────────────────

    pub fn add_relationship(
        &self,
        rel_type: RelationshipType,
        person1_id: PersonId,
        person2_id: PersonId,
        notes: Option<&str>,
    ) -> KinforgeResult<Relationship> {
        let mut rel = Relationship::new(rel_type, person1_id, person2_id);
        rel.notes = notes.map(|s| s.to_string());
        validation::validate_relationship(&rel)?;
        self.db.insert_relationship(&rel)?;
        Ok(rel)
    }

    pub fn delete_relationship(&self, id: &RelationshipId) -> KinforgeResult<()> {
        self.db.delete_relationship(id)
    }

    pub fn list_relationships_for_person(
        &self,
        person_id: &PersonId,
    ) -> KinforgeResult<Vec<Relationship>> {
        self.db.list_relationships_for_person(person_id)
    }

    // ── Sources ─────────────────────────────────────────────────────────────

    pub fn add_source(
        &self,
        title: &str,
        author: Option<&str>,
        publication: Option<&str>,
        year: Option<i32>,
        repository: Option<&str>,
        notes: Option<&str>,
    ) -> KinforgeResult<Source> {
        let mut source = Source::new(title);
        source.author = author.map(|s| s.to_string());
        source.publication = publication.map(|s| s.to_string());
        source.year = year;
        source.repository = repository.map(|s| s.to_string());
        source.notes = notes.map(|s| s.to_string());
        validation::validate_source(&source)?;
        self.db.insert_source(&source)?;
        Ok(source)
    }

    pub fn update_source(&self, source: Source) -> KinforgeResult<Source> {
        validation::validate_source(&source)?;
        self.db.update_source(&source)?;
        Ok(source)
    }

    pub fn delete_source(&self, id: &SourceId) -> KinforgeResult<()> {
        self.db.delete_source(id)
    }

    pub fn get_source(&self, id: &SourceId) -> KinforgeResult<Source> {
        self.db.get_source(id)
    }

    pub fn list_sources(&self) -> KinforgeResult<Vec<Source>> {
        self.db.list_sources()
    }

    // ── Citations ──────────────────────────────────────────────────────────

    pub fn add_citation(
        &self,
        source_id: SourceId,
        event_id: EventId,
        page: Option<&str>,
        confidence: ConfidenceLevel,
        notes: Option<&str>,
    ) -> KinforgeResult<Citation> {
        let mut citation = Citation::new(source_id, event_id);
        citation.page = page.map(|s| s.to_string());
        citation.confidence = confidence;
        citation.notes = notes.map(|s| s.to_string());
        self.db.insert_citation(&citation)?;
        Ok(citation)
    }

    pub fn update_citation(&self, citation: Citation) -> KinforgeResult<Citation> {
        self.db.update_citation(&citation)?;
        Ok(citation)
    }

    pub fn delete_citation(&self, id: &CitationId) -> KinforgeResult<()> {
        self.db.delete_citation(id)
    }

    pub fn list_citations_for_event(&self, event_id: &EventId) -> KinforgeResult<Vec<Citation>> {
        self.db.list_citations_for_event(event_id)
    }
}

// ── Backup ────────────────────────────────────────────────────────────────────

fn create_backup(config: &Config) -> KinforgeResult<()> {
    let src = &config.database_path;
    if !src.exists() {
        return Ok(());
    }

    let backup_dir = src.parent().unwrap_or(Path::new(".")).join("backups");
    std::fs::create_dir_all(&backup_dir)?;

    // Timestamp-based name
    let ts = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    };
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("kinforge");
    let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("db");
    let backup_path = backup_dir.join(format!("{}-{}.{}", stem, ts, ext));

    std::fs::copy(src, &backup_path)?;

    // Prune old backups
    prune_backups(&backup_dir, config.max_backups)?;
    Ok(())
}

fn prune_backups(dir: &Path, max: u32) -> KinforgeResult<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "db").unwrap_or(false))
        .collect();

    // Sort by modification time, oldest first
    entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

    while entries.len() > max as usize {
        let oldest = entries.remove(0);
        std::fs::remove_file(oldest.path())?;
    }
    Ok(())
}
