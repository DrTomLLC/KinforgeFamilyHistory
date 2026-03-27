use chrono::Local;
use kinforge_config::Config;
use kinforge_core::{models::*, validation, KinforgeError, KinforgeResult};
use kinforge_storage::{repository::DatabaseStats, Database, FtsResult};
use std::path::Path;

/// A single issue found during an integrity check.
#[derive(Debug, Clone)]
pub struct IntegrityIssue {
    pub severity: &'static str, // "warning" or "error"
    pub entity_type: String,
    pub id: String,
    pub message: String,
}

/// A single hit from a notes full-text search.
#[derive(Debug, Clone)]
pub struct NotesMatch {
    /// "Person" or "Event"
    pub kind: String,
    /// UUID of the entity
    pub id: String,
    /// Human-readable label (person name, or "Name — EventType")
    pub label: String,
    /// The full notes text that matched
    pub notes: String,
}

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

    /// Edit a name entry in-place by zero-based index.
    ///
    /// Only the supplied fields are changed; pass `None` to leave a field
    /// unchanged.  Pass `Some(None)` to clear a given/surname field.
    pub fn update_name_on_person(
        &self,
        person_id: &PersonId,
        index: usize,
        given: Option<Option<String>>,
        surname: Option<Option<String>>,
        name_type: Option<NameType>,
    ) -> KinforgeResult<Person> {
        let mut person = self.db.get_person(person_id)?;
        let entry = person.names.get_mut(index).ok_or_else(|| {
            KinforgeError::InvalidField {
                field: "name index".to_string(),
                value: index.to_string(),
            }
        })?;
        if let Some(g) = given {
            entry.given = g;
        }
        if let Some(s) = surname {
            entry.surname = s;
        }
        if let Some(nt) = name_type {
            entry.name_type = nt;
        }
        validation::validate_person(&person)?;
        self.db.update_person(&person)?;
        Ok(person)
    }

    /// Remove a name entry by zero-based index.
    ///
    /// The primary name (index 0) can only be removed if there is at least
    /// one other name remaining.
    pub fn delete_name_from_person(
        &self,
        person_id: &PersonId,
        index: usize,
    ) -> KinforgeResult<Person> {
        let mut person = self.db.get_person(person_id)?;
        if person.names.len() <= 1 && index == 0 {
            return Err(KinforgeError::InvalidField {
                field: "name index".to_string(),
                value: "cannot delete the only name; delete the person instead".to_string(),
            });
        }
        if index >= person.names.len() {
            return Err(KinforgeError::InvalidField {
                field: "name index".to_string(),
                value: format!("{} (person has {} name(s))", index, person.names.len()),
            });
        }
        person.names.remove(index);
        self.db.update_person(&person)?;
        Ok(person)
    }

    /// Search person notes and event notes for `query` (case-insensitive).
    ///
    /// Returns a flat list of `(entity_kind, display_label, notes_text)`.
    pub fn search_notes(&self, query: &str) -> KinforgeResult<Vec<NotesMatch>> {
        let q = query.to_lowercase();
        let mut results = Vec::new();

        for person in self.db.list_people()? {
            if let Some(ref notes) = person.notes {
                if notes.to_lowercase().contains(&q) {
                    results.push(NotesMatch {
                        kind: "Person".to_string(),
                        id: person.id.to_string(),
                        label: person.display_name(),
                        notes: notes.clone(),
                    });
                }
            }
            // Also search event notes for this person
            for event in self.db.list_events_for_person(&person.id)? {
                if let Some(ref notes) = event.notes {
                    if notes.to_lowercase().contains(&q) {
                        results.push(NotesMatch {
                            kind: "Event".to_string(),
                            id: event.id.to_string(),
                            label: format!("{} — {}", person.display_name(), event.event_type),
                            notes: notes.clone(),
                        });
                    }
                }
            }
        }

        Ok(results)
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

    // ── ID resolution (full UUID or unambiguous prefix) ────────────────────

    /// Resolve a person ID from a full UUID string or a short unambiguous prefix.
    pub fn resolve_person_id(&self, input: &str) -> KinforgeResult<PersonId> {
        if let Ok(pid) = PersonId::from_str(input) {
            return Ok(pid);
        }
        self.db.find_person_by_id_prefix(input).map(|p| p.id)
    }

    /// Resolve an event ID from a full UUID string or a short unambiguous prefix.
    pub fn resolve_event_id(&self, input: &str) -> KinforgeResult<EventId> {
        if let Ok(eid) = EventId::from_str(input) {
            return Ok(eid);
        }
        self.db.find_event_by_id_prefix(input).map(|e| e.id)
    }

    /// Resolve a place ID from a full UUID string or a short unambiguous prefix.
    pub fn resolve_place_id(&self, input: &str) -> KinforgeResult<PlaceId> {
        if let Ok(pid) = PlaceId::from_str(input) {
            return Ok(pid);
        }
        self.db.find_place_by_id_prefix(input).map(|p| p.id)
    }

    /// Resolve a relationship ID from a full UUID string or a short unambiguous prefix.
    pub fn resolve_relationship_id(&self, input: &str) -> KinforgeResult<RelationshipId> {
        if let Ok(rid) = RelationshipId::from_str(input) {
            return Ok(rid);
        }
        self.db
            .find_relationship_by_id_prefix(input)
            .map(|r| r.id)
    }

    /// Resolve a source ID from a full UUID string or a short unambiguous prefix.
    pub fn resolve_source_id(&self, input: &str) -> KinforgeResult<SourceId> {
        if let Ok(sid) = SourceId::from_str(input) {
            return Ok(sid);
        }
        self.db.find_source_by_id_prefix(input).map(|s| s.id)
    }

    /// Resolve a citation ID from a full UUID string or a short unambiguous prefix.
    pub fn resolve_citation_id(&self, input: &str) -> KinforgeResult<CitationId> {
        if let Ok(cid) = CitationId::from_str(input) {
            return Ok(cid);
        }
        self.db.find_citation_by_id_prefix(input).map(|c| c.id)
    }

    // ── Convenience family-link methods ───────────────────────────────────

    /// Record that `parent_id` is a parent of `child_id`.
    pub fn add_parent(
        &self,
        child_id: PersonId,
        parent_id: PersonId,
        notes: Option<&str>,
    ) -> KinforgeResult<Relationship> {
        self.add_relationship(RelationshipType::ParentChild, parent_id, child_id, notes)
    }

    /// Record that `child_id` is a child of `parent_id`.
    pub fn add_child(
        &self,
        parent_id: PersonId,
        child_id: PersonId,
        notes: Option<&str>,
    ) -> KinforgeResult<Relationship> {
        self.add_relationship(RelationshipType::ParentChild, parent_id, child_id, notes)
    }

    /// Record a spouse relationship between two people.
    pub fn add_spouse(
        &self,
        person1_id: PersonId,
        person2_id: PersonId,
        notes: Option<&str>,
    ) -> KinforgeResult<Relationship> {
        self.add_relationship(RelationshipType::Spouse, person1_id, person2_id, notes)
    }

    /// Merge `source_id` into `target_id`.
    ///
    /// All names unique to source are appended to target.  All events and
    /// relationships belonging to source are reassigned to target.  The source
    /// person record is then deleted.  Returns the updated target person.
    pub fn merge_person(
        &self,
        source_id: &PersonId,
        target_id: &PersonId,
    ) -> KinforgeResult<Person> {
        if source_id == target_id {
            return Err(KinforgeError::Validation(
                "cannot merge a person with themselves".to_string(),
            ));
        }

        let source = self.db.get_person(source_id)?;
        let mut target = self.db.get_person(target_id)?;

        // Copy unique names (match on given + surname).
        for name in &source.names {
            let already = target.names.iter().any(|n| {
                n.given.as_deref() == name.given.as_deref()
                    && n.surname.as_deref() == name.surname.as_deref()
            });
            if !already {
                target.names.push(name.clone());
            }
        }

        // Merge notes: append source notes to target if not already present.
        match (&target.notes, &source.notes) {
            (None, Some(sn)) => target.notes = Some(sn.clone()),
            (Some(tn), Some(sn)) if !tn.contains(sn.as_str()) => {
                target.notes = Some(format!("{}\n{}", tn, sn));
            }
            _ => {}
        }

        self.db.update_person(&target)?;

        // Reassign events and relationships before deleting source.
        self.db.reassign_events_to_person(source_id, target_id)?;
        self.db
            .reassign_relationships_to_person(source_id, target_id)?;

        // Delete source person (cascade removes their person_names only; events/rels moved).
        self.db.delete_person(source_id)?;

        self.db.get_person(target_id)
    }

    // ── Data integrity check ───────────────────────────────────────────────

    /// Run a sweep of the database looking for common data quality problems.
    pub fn check_integrity(&self) -> KinforgeResult<Vec<IntegrityIssue>> {
        let mut issues: Vec<IntegrityIssue> = Vec::new();

        // People with no names
        for person in self.db.list_people()? {
            if person.names.is_empty() {
                issues.push(IntegrityIssue {
                    severity: "error",
                    entity_type: "Person".to_string(),
                    id: person.id.to_string(),
                    message: "has no names".to_string(),
                });
            } else if person.display_name() == "(unnamed)" {
                issues.push(IntegrityIssue {
                    severity: "warning",
                    entity_type: "Person".to_string(),
                    id: person.id.to_string(),
                    message: "primary name has no given or surname".to_string(),
                });
            }

            // People with no events at all
            let events = self.db.list_events_for_person(&person.id)?;
            if events.is_empty() {
                issues.push(IntegrityIssue {
                    severity: "warning",
                    entity_type: "Person".to_string(),
                    id: person.id.to_string(),
                    message: format!(
                        "{} has no events (no birth, death, or any other event)",
                        person.display_name()
                    ),
                });
            }

            // Events with no date
            for event in &events {
                if event.date.is_none() {
                    issues.push(IntegrityIssue {
                        severity: "warning",
                        entity_type: "Event".to_string(),
                        id: event.id.to_string(),
                        message: format!(
                            "{} event for {} has no date",
                            event.event_type,
                            person.display_name()
                        ),
                    });
                }
            }

            // People with no relationships
            let rels = self.db.list_relationships_for_person(&person.id)?;
            if rels.is_empty() {
                issues.push(IntegrityIssue {
                    severity: "warning",
                    entity_type: "Person".to_string(),
                    id: person.id.to_string(),
                    message: format!(
                        "{} has no relationships (not linked to any parent, child, or spouse)",
                        person.display_name()
                    ),
                });
            }

            // Birth date after death date
            let birth_date = events
                .iter()
                .find(|e| matches!(e.event_type, EventType::Birth))
                .and_then(|e| e.date.as_ref())
                .and_then(|d| match d {
                    EventDate::Exact(nd) | EventDate::Approximate(nd) => Some(*nd),
                    _ => None,
                });
            let death_date = events
                .iter()
                .find(|e| matches!(e.event_type, EventType::Death))
                .and_then(|e| e.date.as_ref())
                .and_then(|d| match d {
                    EventDate::Exact(nd) | EventDate::Approximate(nd) => Some(*nd),
                    _ => None,
                });
            if let (Some(b), Some(d)) = (birth_date, death_date) {
                if b > d {
                    issues.push(IntegrityIssue {
                        severity: "error",
                        entity_type: "Person".to_string(),
                        id: person.id.to_string(),
                        message: format!(
                            "{} has birth date ({}) after death date ({})",
                            person.display_name(),
                            b.format("%Y-%m-%d"),
                            d.format("%Y-%m-%d")
                        ),
                    });
                }
            }

            // Duplicate events (same type on same person)
            let mut seen_types: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for event in &events {
                let key = event.event_type.to_string();
                // Only flag duplicates for singleton event types
                let singleton = matches!(
                    event.event_type,
                    EventType::Birth | EventType::Death | EventType::Burial | EventType::Baptism
                );
                if singleton && !seen_types.insert(key.clone()) {
                    issues.push(IntegrityIssue {
                        severity: "warning",
                        entity_type: "Event".to_string(),
                        id: event.id.to_string(),
                        message: format!(
                            "duplicate {} event for {} (multiple {} events recorded)",
                            key,
                            person.display_name(),
                            key
                        ),
                    });
                }
            }
        }

        // Orphan sources: sources with zero citations
        for source in self.db.list_sources()? {
            let citations = self.db.list_citations_for_source(&source.id)?;
            if citations.is_empty() {
                issues.push(IntegrityIssue {
                    severity: "warning",
                    entity_type: "Source".to_string(),
                    id: source.id.to_string(),
                    message: format!(
                        "'{}' has no citations — not referenced by any event",
                        source.title
                    ),
                });
            }
        }

        // Duplicate people: two people with the same given + surname (case-insensitive)
        {
            use std::collections::HashMap;
            let all = self.db.list_people()?;
            // Map normalised "given|surname" -> list of person IDs
            let mut name_map: HashMap<String, Vec<String>> = HashMap::new();
            for person in &all {
                if let Some(name) = person.names.first() {
                    let given = name
                        .given
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase();
                    let surname = name
                        .surname
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase();
                    if !given.is_empty() || !surname.is_empty() {
                        let key = format!("{}|{}", given, surname);
                        name_map.entry(key).or_default().push(person.id.to_string());
                    }
                }
            }
            for (key, ids) in &name_map {
                if ids.len() > 1 {
                    let parts: Vec<&str> = key.splitn(2, '|').collect();
                    let display = format!("{} {}", parts[0], parts[1]).trim().to_string();
                    issues.push(IntegrityIssue {
                        severity: "warning",
                        entity_type: "Person".to_string(),
                        id: ids[0].clone(),
                        message: format!(
                            "possible duplicate: {} people share the name '{}' — consider `person merge`",
                            ids.len(),
                            display
                        ),
                    });
                }
            }
        }

        Ok(issues)
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
        parent_id: Option<PlaceId>,
    ) -> KinforgeResult<Place> {
        let mut place = Place::new(name);
        place.latitude = latitude;
        place.longitude = longitude;
        place.parent_id = parent_id;
        validation::validate_place(&place)?;
        self.db.insert_place(&place)?;
        Ok(place)
    }

    pub fn get_place(&self, id: &PlaceId) -> KinforgeResult<Place> {
        self.db.get_place(id)
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

    pub fn get_relationship(&self, id: &RelationshipId) -> KinforgeResult<Relationship> {
        self.db.get_relationship(id)
    }

    pub fn update_relationship(&self, rel: Relationship) -> KinforgeResult<Relationship> {
        validation::validate_relationship(&rel)?;
        self.db.update_relationship(&rel)?;
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

    pub fn get_citation(&self, id: &CitationId) -> KinforgeResult<Citation> {
        self.db.get_citation(id)
    }

    pub fn list_citations_for_event(&self, event_id: &EventId) -> KinforgeResult<Vec<Citation>> {
        self.db.list_citations_for_event(event_id)
    }

    pub fn list_citations_for_source(
        &self,
        source_id: &SourceId,
    ) -> KinforgeResult<Vec<Citation>> {
        self.db.list_citations_for_source(source_id)
    }

    pub fn list_all_citations(&self) -> KinforgeResult<Vec<Citation>> {
        self.db.list_all_citations()
    }

    // ── Media ────────────────────────────────────────────────────────────────

    pub fn add_media(
        &self,
        title: &str,
        media_type: MediaType,
        path: Option<&str>,
        url: Option<&str>,
        description: Option<&str>,
        date: Option<&str>,
    ) -> KinforgeResult<Media> {
        let mut m = Media::new(title, media_type);
        m.path = path.map(|s| s.to_string());
        m.url = url.map(|s| s.to_string());
        m.description = description.map(|s| s.to_string());
        m.date = date.map(|s| s.to_string());
        self.db.insert_media(&m)?;
        Ok(m)
    }

    pub fn update_media(&self, media: Media) -> KinforgeResult<Media> {
        self.db.update_media(&media)?;
        Ok(media)
    }

    pub fn delete_media(&self, id: &MediaId) -> KinforgeResult<()> {
        self.db.delete_media(id)
    }

    pub fn get_media(&self, id: &MediaId) -> KinforgeResult<Media> {
        self.db.get_media(id)
    }

    pub fn list_media(&self) -> KinforgeResult<Vec<Media>> {
        self.db.list_media()
    }

    /// Attach a media record to a person, event, or source.
    pub fn attach_media(
        &self,
        media_id: &MediaId,
        entity_type: MediaEntityType,
        entity_id: &str,
    ) -> KinforgeResult<MediaLink> {
        // Verify media exists
        self.db.get_media(media_id)?;
        let link = MediaLink::new(media_id.clone(), entity_type, entity_id);
        self.db.insert_media_link(&link)?;
        Ok(link)
    }

    pub fn detach_media(&self, link_id: &MediaLinkId) -> KinforgeResult<()> {
        self.db.delete_media_link(link_id)
    }

    pub fn list_media_for_person(&self, person_id: &PersonId) -> KinforgeResult<Vec<Media>> {
        self.db
            .list_media_for_entity(&MediaEntityType::Person, &person_id.as_str())
    }

    pub fn list_media_for_event(&self, event_id: &EventId) -> KinforgeResult<Vec<Media>> {
        self.db
            .list_media_for_entity(&MediaEntityType::Event, &event_id.as_str())
    }

    pub fn list_media_links_for_media(
        &self,
        media_id: &MediaId,
    ) -> KinforgeResult<Vec<MediaLink>> {
        self.db.list_media_links_for_media(media_id)
    }

    /// Resolve a media ID from a full UUID or unambiguous prefix.
    pub fn resolve_media_id(&self, input: &str) -> KinforgeResult<MediaId> {
        if let Ok(id) = MediaId::from_str(input) {
            return Ok(id);
        }
        let all = self.db.list_media()?;
        let matches: Vec<_> = all
            .iter()
            .filter(|m| m.id.as_str().starts_with(input))
            .collect();
        match matches.len() {
            1 => Ok(matches[0].id.clone()),
            0 => Err(KinforgeError::NotFound {
                entity_type: "Media".to_string(),
                id: input.to_string(),
            }),
            _ => Err(KinforgeError::InvalidField {
                field: "media_id".to_string(),
                value: format!("prefix '{}' is ambiguous ({} matches)", input, matches.len()),
            }),
        }
    }

    // ── Full-text search ──────────────────────────────────────────────────────

    /// Search all text content (names, notes, source titles, place names) using FTS5.
    /// Results are sorted by relevance (best match first).
    pub fn search_fulltext(&self, query: &str) -> KinforgeResult<Vec<FtsResult>> {
        self.db.search_fulltext(query)
    }

    // ── Relationship path finding ─────────────────────────────────────────────

    /// BFS through the relationship graph to find the shortest path between two people.
    /// Returns `None` if no connection exists.
    pub fn find_relationship_path(
        &self,
        from_id: &PersonId,
        to_id: &PersonId,
    ) -> KinforgeResult<Option<RelationshipPath>> {
        use std::collections::{HashMap, VecDeque};

        if from_id == to_id {
            let person = self.db.get_person(from_id)?;
            return Ok(Some(RelationshipPath {
                steps: vec![RelationshipStep {
                    person,
                    via_rel_type: None,
                    direction: None,
                }],
            }));
        }

        // BFS state: entity_id_str -> (parent_id_str, rel_type, person1_was_current)
        let mut parent: HashMap<String, (String, RelationshipType, bool)> = HashMap::new();
        let mut queue: VecDeque<PersonId> = VecDeque::new();

        parent.insert(from_id.as_str(), ("".to_string(), RelationshipType::Sibling, false));
        queue.push_back(from_id.clone());
        let mut found = false;

        'bfs: while let Some(current) = queue.pop_front() {
            let rels = self.db.list_relationships_for_person(&current)?;
            for rel in rels {
                let neighbor = if rel.person1_id == current {
                    rel.person2_id.clone()
                } else {
                    rel.person1_id.clone()
                };
                let nkey = neighbor.as_str();
                if parent.contains_key(&nkey) {
                    continue;
                }
                parent.insert(
                    nkey.clone(),
                    (current.as_str(), rel.rel_type.clone(), rel.person1_id == current),
                );
                if neighbor == *to_id {
                    found = true;
                    break 'bfs;
                }
                queue.push_back(neighbor);
            }
        }

        if !found {
            return Ok(None);
        }

        // Reconstruct path: walk backwards from to_id to from_id
        let mut path_ids: Vec<(String, Option<RelationshipType>, Option<bool>)> = Vec::new();
        let mut current_key = to_id.as_str();
        loop {
            let (prev_key, rel_type, p1_was_prev) = parent.get(&current_key).unwrap();
            if prev_key.is_empty() {
                path_ids.push((current_key.clone(), None, None));
                break;
            }
            path_ids.push((
                current_key.clone(),
                Some(rel_type.clone()),
                Some(*p1_was_prev),
            ));
            current_key = prev_key.clone();
        }
        path_ids.reverse();

        // Build RelationshipStep list
        let mut steps: Vec<RelationshipStep> = Vec::new();
        for (id_str, via_rel, p1_was_prev) in path_ids {
            let pid = PersonId::from_str(&id_str)
                .map_err(|e| KinforgeError::Storage(e.to_string()))?;
            let person = self.db.get_person(&pid)?;
            let direction = p1_was_prev.map(|was_p1| {
                if was_p1 {
                    RelationshipDirection::Person1ToPerson2
                } else {
                    RelationshipDirection::Person2ToPerson1
                }
            });
            steps.push(RelationshipStep {
                person,
                via_rel_type: via_rel,
                direction,
            });
        }

        Ok(Some(RelationshipPath { steps }))
    }
}

/// One step in a relationship path between two people.
#[derive(Debug, Clone)]
pub struct RelationshipStep {
    pub person: Person,
    /// The relationship that connected the previous step to this person (None for the start node)
    pub via_rel_type: Option<RelationshipType>,
    /// Whether the previous person was `person1` (→ person2) or `person2` (← person1)
    pub direction: Option<RelationshipDirection>,
}

/// Direction of a relationship in a path step.
#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipDirection {
    Person1ToPerson2,
    Person2ToPerson1,
}

/// The complete path between two people through the relationship graph.
#[derive(Debug, Clone)]
pub struct RelationshipPath {
    pub steps: Vec<RelationshipStep>,
}

impl RelationshipPath {
    /// Describe each hop in human-readable form.
    pub fn describe(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for (i, step) in self.steps.iter().enumerate() {
            if i == 0 {
                lines.push(step.person.display_name());
            } else {
                let rel_desc = if let (Some(ref rt), Some(ref dir)) =
                    (&step.via_rel_type, &step.direction)
                {
                    describe_path_rel(rt, dir)
                } else {
                    "related to".to_string()
                };
                lines.push(format!("  └─ {} → {}", rel_desc, step.person.display_name()));
            }
        }
        lines
    }
}

fn describe_path_rel(rt: &RelationshipType, dir: &RelationshipDirection) -> String {
    use RelationshipDirection::*;
    match rt {
        RelationshipType::ParentChild => match dir {
            Person1ToPerson2 => "parent of".to_string(),
            Person2ToPerson1 => "child of".to_string(),
        },
        RelationshipType::AdoptiveParent => match dir {
            Person1ToPerson2 => "adoptive parent of".to_string(),
            Person2ToPerson1 => "adoptive child of".to_string(),
        },
        RelationshipType::StepParent => match dir {
            Person1ToPerson2 => "step-parent of".to_string(),
            Person2ToPerson1 => "step-child of".to_string(),
        },
        RelationshipType::Godparent => match dir {
            Person1ToPerson2 => "godparent of".to_string(),
            Person2ToPerson1 => "godchild of".to_string(),
        },
        RelationshipType::Foster => match dir {
            Person1ToPerson2 => "foster parent of".to_string(),
            Person2ToPerson1 => "foster child of".to_string(),
        },
        RelationshipType::Spouse => "spouse of".to_string(),
        RelationshipType::Sibling => "sibling of".to_string(),
        RelationshipType::HalfSibling => "half-sibling of".to_string(),
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

    // Human-readable timestamp: YYYY-MM-DD_HH-MM-SS
    let ts = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("kinforge");
    let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("db");
    let backup_path = backup_dir.join(format!("{}_{}.{}", stem, ts, ext));

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
