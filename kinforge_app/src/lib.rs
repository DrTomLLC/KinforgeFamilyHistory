use kinforge_config::Config;
use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;

pub struct Application {
    pub db: Database,
    pub config: Config,
}

impl Application {
    pub fn open(config: Config) -> KinforgeResult<Self> {
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
        self.db.insert_person(&person)?;
        Ok(person)
    }

    pub fn get_person(&self, id: &PersonId) -> KinforgeResult<Person> {
        self.db.get_person(id)
    }

    pub fn list_people(&self) -> KinforgeResult<Vec<Person>> {
        self.db.list_people()
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
            self.db.insert_place(&place)?;
            Some(place.id)
        } else {
            None
        };

        let mut event = Event::new(event_type, person_id);
        event.date = date;
        event.place_id = place_id;
        event.notes = notes.map(|s| s.to_string());
        self.db.insert_event(&event)?;
        Ok(event)
    }

    pub fn list_events_for_person(&self, person_id: &PersonId) -> KinforgeResult<Vec<Event>> {
        self.db.list_events_for_person(person_id)
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
        self.db.insert_relationship(&rel)?;
        Ok(rel)
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
        self.db.insert_source(&source)?;
        Ok(source)
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
}
