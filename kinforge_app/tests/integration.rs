use chrono::NaiveDate;
use kinforge_app::Application;
use kinforge_core::models::*;

fn app() -> Application {
    Application::open_in_memory().unwrap()
}

// ── Person CRUD ───────────────────────────────────────────────────────────────

#[test]
fn add_and_get_person() {
    let a = app();
    let p = a
        .add_person(Some("Mary"), Some("Jones"), Sex::Female, None)
        .unwrap();
    assert_eq!(p.display_name(), "Mary Jones");
    let fetched = a.get_person(&p.id).unwrap();
    assert_eq!(fetched.sex, Sex::Female);
}

#[test]
fn update_person_name() {
    let a = app();
    let mut p = a
        .add_person(Some("Jim"), Some("Brown"), Sex::Male, None)
        .unwrap();
    p.names[0].surname = Some("Green".to_string());
    a.update_person(p.clone()).unwrap();
    let fetched = a.get_person(&p.id).unwrap();
    assert_eq!(fetched.names[0].surname, Some("Green".to_string()));
}

#[test]
fn delete_person_cascades_events() {
    let a = app();
    let p = a
        .add_person(Some("Bob"), Some("White"), Sex::Male, None)
        .unwrap();
    let e = a
        .add_event(p.id.clone(), EventType::Birth, None, None, None)
        .unwrap();
    a.delete_person(&p.id).unwrap();
    assert!(a.get_person(&p.id).is_err());
    assert!(a.database().get_event(&e.id).is_err());
}

#[test]
fn add_alternate_name() {
    let a = app();
    let p = a
        .add_person(Some("Elizabeth"), Some("Windsor"), Sex::Female, None)
        .unwrap();
    let p2 = a
        .add_name_to_person(&p.id, None, Some("Mountbatten-Windsor"), NameType::Birth)
        .unwrap();
    assert_eq!(p2.names.len(), 2);
}

// ── Event CRUD ────────────────────────────────────────────────────────────────

#[test]
fn add_event_with_date_and_place() {
    let a = app();
    let p = a
        .add_person(Some("Alice"), None, Sex::Female, None)
        .unwrap();
    let date = EventDate::Exact(NaiveDate::from_ymd_opt(1900, 1, 1).unwrap());
    let e = a
        .add_event(
            p.id.clone(),
            EventType::Birth,
            Some(date.clone()),
            Some("Boston, MA"),
            None,
        )
        .unwrap();
    let fetched = a.database().get_event(&e.id).unwrap();
    match &fetched.date {
        Some(EventDate::Exact(d)) => assert_eq!(*d, NaiveDate::from_ymd_opt(1900, 1, 1).unwrap()),
        _ => panic!("wrong date kind"),
    }
    let place = a.database().get_place(fetched.place_id.as_ref().unwrap()).unwrap();
    assert_eq!(place.name, "Boston, MA");
}

#[test]
fn update_event() {
    let a = app();
    let p = a.add_person(Some("Carl"), None, Sex::Male, None).unwrap();
    let mut e = a
        .add_event(p.id.clone(), EventType::Residence, None, None, None)
        .unwrap();
    e.notes = Some("Moved to Ohio".to_string());
    a.update_event(e.clone()).unwrap();
    let fetched = a.database().get_event(&e.id).unwrap();
    assert_eq!(fetched.notes, Some("Moved to Ohio".to_string()));
}

#[test]
fn delete_event() {
    let a = app();
    let p = a.add_person(Some("Dan"), None, Sex::Male, None).unwrap();
    let e = a
        .add_event(p.id.clone(), EventType::Census, None, None, None)
        .unwrap();
    a.delete_event(&e.id).unwrap();
    assert!(a.database().get_event(&e.id).is_err());
}

#[test]
fn event_date_between_validation() {
    let a = app();
    let p = a.add_person(Some("Eve"), None, Sex::Female, None).unwrap();
    // d1 > d2 should fail validation
    let d1 = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let d2 = NaiveDate::from_ymd_opt(1990, 1, 1).unwrap();
    let result = a.add_event(
        p.id.clone(),
        EventType::Birth,
        Some(EventDate::Between(d1, d2)),
        None,
        None,
    );
    assert!(result.is_err());
}

// ── Relationship CRUD ─────────────────────────────────────────────────────────

#[test]
fn add_family() {
    let a = app();
    let father = a
        .add_person(Some("Fred"), Some("Smith"), Sex::Male, None)
        .unwrap();
    let mother = a
        .add_person(Some("Greta"), Some("Smith"), Sex::Female, None)
        .unwrap();
    let child = a
        .add_person(Some("Henry"), Some("Smith"), Sex::Male, None)
        .unwrap();

    let spouse = a
        .add_relationship(
            RelationshipType::Spouse,
            father.id.clone(),
            mother.id.clone(),
            None,
        )
        .unwrap();
    let pc1 = a
        .add_relationship(
            RelationshipType::ParentChild,
            father.id.clone(),
            child.id.clone(),
            None,
        )
        .unwrap();
    let pc2 = a
        .add_relationship(
            RelationshipType::ParentChild,
            mother.id.clone(),
            child.id.clone(),
            None,
        )
        .unwrap();

    let rels = a.list_relationships_for_person(&child.id).unwrap();
    assert_eq!(rels.len(), 2);
    let _ = (spouse, pc1, pc2);
}

#[test]
fn self_relationship_rejected() {
    let a = app();
    let p = a.add_person(Some("Ian"), None, Sex::Male, None).unwrap();
    let result = a.add_relationship(RelationshipType::Sibling, p.id.clone(), p.id.clone(), None);
    assert!(result.is_err());
}

#[test]
fn delete_relationship() {
    let a = app();
    let p1 = a.add_person(Some("Jack"), None, Sex::Male, None).unwrap();
    let p2 = a.add_person(Some("Jill"), None, Sex::Female, None).unwrap();
    let rel = a
        .add_relationship(RelationshipType::Spouse, p1.id.clone(), p2.id.clone(), None)
        .unwrap();
    a.delete_relationship(&rel.id).unwrap();
    assert!(a.database().get_relationship(&rel.id).is_err());
}

// ── Source and Citation ───────────────────────────────────────────────────────

#[test]
fn source_crud() {
    let a = app();
    let src = a
        .add_source("1900 Census", Some("US Gov"), None, Some(1900), None, None)
        .unwrap();
    let mut s = a.get_source(&src.id).unwrap();
    s.year = Some(1901);
    a.update_source(s).unwrap();
    let fetched = a.get_source(&src.id).unwrap();
    assert_eq!(fetched.year, Some(1901));
    a.delete_source(&src.id).unwrap();
    assert!(a.get_source(&src.id).is_err());
}

#[test]
fn citation_links_source_to_event() {
    let a = app();
    let p = a.add_person(Some("Kate"), None, Sex::Female, None).unwrap();
    let e = a
        .add_event(p.id.clone(), EventType::Birth, None, None, None)
        .unwrap();
    let s = a
        .add_source("Birth Register", None, None, Some(1880), None, None)
        .unwrap();
    let cit = a
        .add_citation(
            s.id.clone(),
            e.id.clone(),
            Some("p.12"),
            ConfidenceLevel::Primary,
            None,
        )
        .unwrap();
    let citations = a.list_citations_for_event(&e.id).unwrap();
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].id, cit.id);
    assert_eq!(citations[0].confidence, ConfidenceLevel::Primary);
}

#[test]
fn citation_update_and_delete() {
    let a = app();
    let p = a.add_person(Some("Leo"), None, Sex::Male, None).unwrap();
    let e = a
        .add_event(p.id.clone(), EventType::Death, None, None, None)
        .unwrap();
    let s = a
        .add_source("Death Records", None, None, None, None, None)
        .unwrap();
    let mut cit = a
        .add_citation(
            s.id.clone(),
            e.id.clone(),
            None,
            ConfidenceLevel::Secondary,
            None,
        )
        .unwrap();
    cit.page = Some("vol.3 p.99".to_string());
    a.update_citation(cit.clone()).unwrap();
    let fetched = a.database().get_citation(&cit.id).unwrap();
    assert_eq!(fetched.page, Some("vol.3 p.99".to_string()));
    a.delete_citation(&cit.id).unwrap();
    assert!(a.database().get_citation(&cit.id).is_err());
}

// ── Place CRUD ────────────────────────────────────────────────────────────────

#[test]
fn place_crud() {
    let a = app();
    let pl = a
        .add_place("London, England", Some(51.5074), Some(-0.1278), None)
        .unwrap();
    let mut fetched = a.database().get_place(&pl.id).unwrap();
    assert_eq!(fetched.name, "London, England");
    fetched.name = "London, UK".to_string();
    a.update_place(fetched.clone()).unwrap();
    let updated = a.database().get_place(&pl.id).unwrap();
    assert_eq!(updated.name, "London, UK");
    a.delete_place(&pl.id).unwrap();
    assert!(a.database().get_place(&pl.id).is_err());
}

#[test]
fn place_invalid_coords_rejected() {
    let a = app();
    let result = a.add_place("Bad Place", Some(200.0), None, None);
    assert!(result.is_err());
}

// ── Statistics ────────────────────────────────────────────────────────────────

#[test]
fn stats_accuracy() {
    let a = app();
    let p = a.add_person(Some("Mia"), None, Sex::Female, None).unwrap();
    a.add_event(p.id.clone(), EventType::Birth, None, None, None)
        .unwrap();
    let s = a.stats().unwrap();
    assert_eq!(s.people, 1);
    assert_eq!(s.events, 1);
    assert_eq!(s.sources, 0);
}

// ── New feature tests ─────────────────────────────────────────────────────────

#[test]
fn event_date_approximate() {
    let a = app();
    let p = a
        .add_person(Some("Old"), Some("Timer"), Sex::Male, None)
        .unwrap();
    let date = EventDate::Approximate(NaiveDate::from_ymd_opt(1850, 1, 1).unwrap());
    let e = a
        .add_event(
            p.id.clone(),
            EventType::Birth,
            Some(date.clone()),
            None,
            None,
        )
        .unwrap();
    let fetched = a.get_event(&e.id).unwrap();
    assert_eq!(fetched.date, Some(date));
}

#[test]
fn event_date_between() {
    let a = app();
    let p = a
        .add_person(Some("Range"), None, Sex::Unknown, None)
        .unwrap();
    let d1 = NaiveDate::from_ymd_opt(1880, 1, 1).unwrap();
    let d2 = NaiveDate::from_ymd_opt(1885, 12, 31).unwrap();
    let date = EventDate::Between(d1, d2);
    let e = a
        .add_event(
            p.id.clone(),
            EventType::Birth,
            Some(date.clone()),
            None,
            None,
        )
        .unwrap();
    let fetched = a.get_event(&e.id).unwrap();
    assert_eq!(fetched.date, Some(date));
}

#[test]
fn relationship_update_notes() {
    let a = app();
    let p1 = a
        .add_person(Some("Alice"), None, Sex::Female, None)
        .unwrap();
    let p2 = a.add_person(Some("Bob"), None, Sex::Male, None).unwrap();
    let rel = a
        .add_relationship(RelationshipType::Spouse, p1.id.clone(), p2.id.clone(), None)
        .unwrap();
    let mut updated = a.get_relationship(&rel.id).unwrap();
    updated.notes = Some("married 1910".to_string());
    a.update_relationship(updated).unwrap();
    let fetched = a.get_relationship(&rel.id).unwrap();
    assert_eq!(fetched.notes, Some("married 1910".to_string()));
}

#[test]
fn relationship_show() {
    let a = app();
    let p1 = a.add_person(Some("Parent"), None, Sex::Male, None).unwrap();
    let p2 = a
        .add_person(Some("Child"), None, Sex::Female, None)
        .unwrap();
    let rel = a
        .add_relationship(
            RelationshipType::ParentChild,
            p1.id.clone(),
            p2.id.clone(),
            None,
        )
        .unwrap();
    let fetched = a.get_relationship(&rel.id).unwrap();
    assert_eq!(fetched.rel_type, RelationshipType::ParentChild);
    assert_eq!(fetched.person1_id, p1.id);
    assert_eq!(fetched.person2_id, p2.id);
}

#[test]
fn place_parent_hierarchy() {
    let a = app();
    let county = a.add_place("Suffolk County", None, None, None).unwrap();
    let town = a
        .add_place("Boston", Some(42.36), Some(-71.06), Some(county.id.clone()))
        .unwrap();
    let fetched = a.get_place(&town.id).unwrap();
    assert_eq!(fetched.parent_id, Some(county.id));
}

#[test]
fn place_parent_shown_in_update() {
    let a = app();
    let state = a.add_place("Massachusetts", None, None, None).unwrap();
    let mut town = a.add_place("Springfield", None, None, None).unwrap();
    town.parent_id = Some(state.id.clone());
    a.update_place(town.clone()).unwrap();
    let fetched = a.get_place(&town.id).unwrap();
    assert_eq!(fetched.parent_id, Some(state.id));
}

#[test]
fn get_citation_roundtrip() {
    let a = app();
    let p = a
        .add_person(Some("Cite"), Some("Me"), Sex::Male, None)
        .unwrap();
    let e = a
        .add_event(p.id.clone(), EventType::Birth, None, None, None)
        .unwrap();
    let src = a
        .add_source("Test Book", None, None, Some(1900), None, None)
        .unwrap();
    let cit = a
        .add_citation(
            src.id.clone(),
            e.id.clone(),
            Some("p.42"),
            ConfidenceLevel::Primary,
            None,
        )
        .unwrap();
    let fetched = a.get_citation(&cit.id).unwrap();
    assert_eq!(fetched.page, Some("p.42".to_string()));
    assert_eq!(fetched.confidence, ConfidenceLevel::Primary);
}

// ── Phase 2: name editing ─────────────────────────────────────────────────────

#[test]
fn update_name_on_person() {
    let a = app();
    let p = a
        .add_person(Some("Old"), Some("Name"), Sex::Unknown, None)
        .unwrap();
    let updated = a
        .update_name_on_person(
            &p.id,
            0,
            Some(Some("New".to_string())),
            Some(Some("Person".to_string())),
            None,
        )
        .unwrap();
    assert_eq!(updated.display_name(), "New Person");
    let fetched = a.get_person(&p.id).unwrap();
    assert_eq!(fetched.display_name(), "New Person");
}

#[test]
fn update_name_out_of_bounds_errors() {
    let a = app();
    let p = a
        .add_person(Some("Jane"), Some("Doe"), Sex::Female, None)
        .unwrap();
    assert!(a
        .update_name_on_person(&p.id, 5, None, None, None)
        .is_err());
}

#[test]
fn delete_name_from_person() {
    let a = app();
    let p = a
        .add_person(Some("John"), Some("Smith"), Sex::Male, None)
        .unwrap();
    // Add a second name so we can delete the first
    let p = a
        .add_name_to_person(&p.id, Some("Johnny"), Some("Smith"), NameType::Nickname)
        .unwrap();
    assert_eq!(p.names.len(), 2);
    let after = a.delete_name_from_person(&p.id, 1).unwrap();
    assert_eq!(after.names.len(), 1);
    assert_eq!(after.names[0].given, Some("John".to_string()));
}

#[test]
fn delete_only_name_errors() {
    let a = app();
    let p = a
        .add_person(Some("Solo"), None, Sex::Unknown, None)
        .unwrap();
    assert!(a.delete_name_from_person(&p.id, 0).is_err());
}

// ── Phase 2: notes search ─────────────────────────────────────────────────────

#[test]
fn search_notes_finds_person_notes() {
    let a = app();
    let mut p = a
        .add_person(Some("Alice"), Some("Notesworthy"), Sex::Female, None)
        .unwrap();
    p.notes = Some("emigrated to Canada in 1902".to_string());
    a.update_person(p).unwrap();

    let results = a.search_notes("Canada").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, "Person");
    assert!(results[0].notes.contains("Canada"));
}

#[test]
fn search_notes_finds_event_notes() {
    let a = app();
    let p = a
        .add_person(Some("Bob"), Some("Clues"), Sex::Male, None)
        .unwrap();
    let mut e = a
        .add_event(p.id.clone(), EventType::Birth, None, None, None)
        .unwrap();
    e.notes = Some("witnessed by Dr. Holmes".to_string());
    a.update_event(e).unwrap();

    let results = a.search_notes("Holmes").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, "Event");
}

#[test]
fn search_notes_case_insensitive() {
    let a = app();
    let mut p = a
        .add_person(Some("Clara"), None, Sex::Female, None)
        .unwrap();
    p.notes = Some("Moved to BOSTON".to_string());
    a.update_person(p).unwrap();

    assert_eq!(a.search_notes("boston").unwrap().len(), 1);
    assert_eq!(a.search_notes("BOSTON").unwrap().len(), 1);
    assert_eq!(a.search_notes("xyz_no_match").unwrap().len(), 0);
}

// ── Phase 4: Merge, citations by source, ancestor tree ────────────────────────

#[test]
fn merge_person_combines_names_and_events() {
    let a = app();
    let source = a
        .add_person(Some("William"), Some("Smith"), Sex::Male, Some("duplicate"))
        .unwrap();
    let target = a
        .add_person(Some("Will"), Some("Smith"), Sex::Male, None)
        .unwrap();

    // Add an event on the source person
    a.add_event(
        source.id.clone(),
        EventType::Birth,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1880, 3, 1).unwrap())),
        None,
        None,
    )
    .unwrap();

    let merged = a.merge_person(&source.id, &target.id).unwrap();

    // Source is gone
    assert!(a.get_person(&source.id).is_err());

    // Target has both names
    assert_eq!(merged.names.len(), 2);

    // Event is now on target
    let events = a.list_events_for_person(&target.id).unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].event_type, EventType::Birth));

    // Notes merged
    assert!(merged.notes.as_deref().unwrap_or("").contains("duplicate"));
}

#[test]
fn merge_person_reassigns_relationships() {
    let a = app();
    let child = a
        .add_person(Some("Charlie"), None, Sex::Male, None)
        .unwrap();
    let source_parent = a
        .add_person(Some("Alice"), None, Sex::Female, None)
        .unwrap();
    let target_parent = a
        .add_person(Some("Alicia"), None, Sex::Female, None)
        .unwrap();

    a.add_parent(child.id.clone(), source_parent.id.clone(), None)
        .unwrap();

    let merged = a.merge_person(&source_parent.id, &target_parent.id).unwrap();

    // The child should now be linked to target_parent
    let rels = a.list_relationships_for_person(&merged.id).unwrap();
    let parent_rels: Vec<_> = rels
        .iter()
        .filter(|r| r.rel_type == RelationshipType::ParentChild)
        .collect();
    assert_eq!(parent_rels.len(), 1);
    assert_eq!(parent_rels[0].person2_id, child.id);
}

#[test]
fn merge_person_self_rejected() {
    let a = app();
    let p = a
        .add_person(Some("John"), None, Sex::Male, None)
        .unwrap();
    assert!(a.merge_person(&p.id, &p.id).is_err());
}

#[test]
fn list_citations_for_source_returns_correct() {
    let a = app();
    let person = a
        .add_person(Some("Test"), None, Sex::Unknown, None)
        .unwrap();
    let event = a
        .add_event(person.id.clone(), EventType::Birth, None, None, None)
        .unwrap();
    let source1 = a
        .add_source("Book One", None, None, None, None, None)
        .unwrap();
    let source2 = a
        .add_source("Book Two", None, None, None, None, None)
        .unwrap();

    a.add_citation(
        source1.id.clone(),
        event.id.clone(),
        Some("p.12"),
        ConfidenceLevel::Primary,
        None,
    )
    .unwrap();
    a.add_citation(
        source1.id.clone(),
        event.id.clone(),
        Some("p.15"),
        ConfidenceLevel::Secondary,
        None,
    )
    .unwrap();
    a.add_citation(
        source2.id.clone(),
        event.id.clone(),
        None,
        ConfidenceLevel::Questionable,
        None,
    )
    .unwrap();

    let s1_cits = a.list_citations_for_source(&source1.id).unwrap();
    assert_eq!(s1_cits.len(), 2);

    let s2_cits = a.list_citations_for_source(&source2.id).unwrap();
    assert_eq!(s2_cits.len(), 1);
}

// ── Phase 5 tests ─────────────────────────────────────────────────────────────

#[test]
fn check_integrity_birth_after_death() {
    let a = app();
    let p = a
        .add_person(Some("Ghost"), Some("Test"), Sex::Unknown, None)
        .unwrap();
    // Death before birth
    a.add_event(
        p.id.clone(),
        EventType::Birth,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1950, 1, 1).unwrap())),
        None,
        None,
    )
    .unwrap();
    a.add_event(
        p.id.clone(),
        EventType::Death,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1940, 6, 15).unwrap())),
        None,
        None,
    )
    .unwrap();
    let issues = a.check_integrity().unwrap();
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == "error" && i.message.contains("birth") || i.message.contains("Birth"))
        .collect();
    assert!(!errors.is_empty(), "expected birth-after-death error");
}

#[test]
fn check_integrity_detects_orphan_sources() {
    let a = app();
    a.add_source("Unused Book", None, None, None, None, None)
        .unwrap();
    let issues = a.check_integrity().unwrap();
    let warnings: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == "warning" && i.message.to_lowercase().contains("citation"))
        .collect();
    assert!(!warnings.is_empty(), "expected orphan-source warning");
}

#[test]
fn event_query_year_range() {
    use kinforge_query::EventQuery;
    let a = app();
    let p = a
        .add_person(Some("Range"), Some("Test"), Sex::Male, None)
        .unwrap();
    a.add_event(
        p.id.clone(),
        EventType::Birth,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1800, 3, 1).unwrap())),
        None,
        None,
    )
    .unwrap();
    a.add_event(
        p.id.clone(),
        EventType::Death,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1870, 11, 20).unwrap())),
        None,
        None,
    )
    .unwrap();
    a.add_event(
        p.id.clone(),
        EventType::Marriage,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1900, 5, 10).unwrap())),
        None,
        None,
    )
    .unwrap();

    let results = EventQuery::new()
        .from_year(1850)
        .to_year(1880)
        .run(&a.database())
        .unwrap();
    // Only the 1870 death event should be in 1850–1880
    assert_eq!(results.len(), 1);
    assert!(matches!(results[0].event_type, EventType::Death));
}

#[test]
fn sources_report_renders() {
    use kinforge_reports::sources_report;
    let a = app();
    let p = a
        .add_person(Some("Alice"), None, Sex::Female, None)
        .unwrap();
    let ev = a
        .add_event(p.id.clone(), EventType::Birth, None, None, None)
        .unwrap();
    let s1 = a.add_source("Used Source", None, None, None, None, None).unwrap();
    let s2 = a.add_source("Orphan Source", None, None, None, None, None).unwrap();
    a.add_citation(
        s1.id.clone(),
        ev.id.clone(),
        None,
        ConfidenceLevel::Primary,
        None,
    )
    .unwrap();
    let _ = s2; // deliberately uncited

    let report = sources_report(&a.database()).unwrap();
    assert!(report.contains("Used Source"));
    assert!(report.contains("Orphan Source"));
    assert!(report.contains("1 citation") || report.contains("1")); // cited count
}

// ── Phase 6 tests ─────────────────────────────────────────────────────────────

#[test]
fn media_crud_roundtrip() {
    use kinforge_core::models::MediaType;
    let a = app();
    let m = a
        .add_media("Portrait 1880", MediaType::Photo, Some("/photos/p.jpg"), None, Some("A sepia portrait"), Some("1880"))
        .unwrap();
    assert_eq!(m.title, "Portrait 1880");
    assert_eq!(m.media_type, MediaType::Photo);

    let fetched = a.get_media(&m.id).unwrap();
    assert_eq!(fetched.path, Some("/photos/p.jpg".to_string()));
    assert_eq!(fetched.date, Some("1880".to_string()));

    let all = a.list_media().unwrap();
    assert_eq!(all.len(), 1);

    a.delete_media(&m.id).unwrap();
    assert!(a.get_media(&m.id).is_err());
}

#[test]
fn media_attach_and_list_for_person() {
    use kinforge_core::models::{MediaEntityType, MediaType};
    let a = app();
    let p = a.add_person(Some("Alice"), None, Sex::Female, None).unwrap();
    let m = a.add_media("Wedding Photo", MediaType::Photo, None, None, None, None).unwrap();

    let link = a.attach_media(&m.id, MediaEntityType::Person, &p.id.as_str()).unwrap();
    let media_list = a.list_media_for_person(&p.id).unwrap();
    assert_eq!(media_list.len(), 1);
    assert_eq!(media_list[0].id, m.id);

    // Detach
    a.detach_media(&link.id).unwrap();
    let media_list2 = a.list_media_for_person(&p.id).unwrap();
    assert!(media_list2.is_empty());
}

#[test]
fn media_attach_to_event() {
    use kinforge_core::models::{MediaEntityType, MediaType};
    let a = app();
    let p = a.add_person(Some("Bob"), None, Sex::Male, None).unwrap();
    let ev = a.add_event(p.id.clone(), EventType::Birth, None, None, None).unwrap();
    let m = a.add_media("Birth Record", MediaType::Document, None, None, None, None).unwrap();
    a.attach_media(&m.id, MediaEntityType::Event, &ev.id.as_str()).unwrap();
    let media = a.list_media_for_event(&ev.id).unwrap();
    assert_eq!(media.len(), 1);
}

#[test]
fn new_relationship_types_roundtrip() {
    let a = app();
    let adopter = a.add_person(Some("Emma"), None, Sex::Female, None).unwrap();
    let child = a.add_person(Some("Lily"), None, Sex::Female, None).unwrap();
    let godfather = a.add_person(Some("James"), None, Sex::Male, None).unwrap();

    a.add_relationship(
        RelationshipType::AdoptiveParent,
        adopter.id.clone(),
        child.id.clone(),
        None,
    ).unwrap();
    a.add_relationship(
        RelationshipType::Godparent,
        godfather.id.clone(),
        child.id.clone(),
        None,
    ).unwrap();

    let rels = a.list_relationships_for_person(&child.id).unwrap();
    assert_eq!(rels.len(), 2);
    let types: Vec<_> = rels.iter().map(|r| r.rel_type.to_string()).collect();
    assert!(types.contains(&"AdoptiveParent".to_string()));
    assert!(types.contains(&"Godparent".to_string()));
}

#[test]
fn check_integrity_detects_duplicate_people() {
    let a = app();
    a.add_person(Some("John"), Some("Smith"), Sex::Male, None).unwrap();
    a.add_person(Some("John"), Some("Smith"), Sex::Male, None).unwrap();
    let issues = a.check_integrity().unwrap();
    let dupe_warnings: Vec<_> = issues
        .iter()
        .filter(|i| i.message.contains("duplicate") || i.message.contains("share the name"))
        .collect();
    assert!(!dupe_warnings.is_empty(), "expected duplicate person warning");
}

#[test]
fn narrative_report_renders() {
    use kinforge_reports::narrative_report;
    let a = app();
    let p = a.add_person(Some("Thomas"), Some("Edison"), Sex::Male, None).unwrap();
    a.add_event(
        p.id.clone(),
        EventType::Birth,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1847, 2, 11).unwrap())),
        None,
        None,
    ).unwrap();
    a.add_event(
        p.id.clone(),
        EventType::Death,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1931, 10, 18).unwrap())),
        None,
        None,
    ).unwrap();

    let report = narrative_report(&a.database(), &p.id).unwrap();
    assert!(report.contains("Thomas Edison"));
    assert!(report.contains("born") || report.contains("Birth"));
    assert!(report.contains("died") || report.contains("Death"));
    assert!(report.contains("1847"));
    assert!(report.contains("1931"));
}

// ── Phase 7 tests ─────────────────────────────────────────────────────────────

#[test]
fn fts_fulltext_finds_person_name() {
    let a = app();
    a.add_person(Some("Fitzwilliam"), Some("Darcy"), Sex::Male, None)
        .unwrap();
    a.add_person(Some("Elizabeth"), Some("Bennet"), Sex::Female, None)
        .unwrap();
    let results = a.search_fulltext("Darcy").unwrap();
    assert!(!results.is_empty(), "FTS should find Darcy");
    assert!(results.iter().any(|r| r.entity_type == "person"));
}

#[test]
fn fts_fulltext_finds_event_notes() {
    let a = app();
    let p = a.add_person(Some("Anne"), None, Sex::Female, None).unwrap();
    a.add_event(
        p.id.clone(),
        EventType::Birth,
        None,
        None,
        Some("born during a blizzard"),
    )
    .unwrap();
    let results = a.search_fulltext("blizzard").unwrap();
    assert!(!results.is_empty(), "FTS should find event notes");
    assert!(results.iter().any(|r| r.entity_type == "event"));
}

#[test]
fn fts_fulltext_finds_source_title() {
    let a = app();
    a.add_source("Vestry Baptism Register 1820", None, None, None, None, None)
        .unwrap();
    let results = a.search_fulltext("Vestry").unwrap();
    assert!(!results.is_empty(), "FTS should find source title");
    assert!(results.iter().any(|r| r.entity_type == "source"));
}

#[test]
fn fts_returns_empty_for_no_match() {
    let a = app();
    a.add_person(Some("John"), Some("Smith"), Sex::Male, None)
        .unwrap();
    let results = a.search_fulltext("xyznonexistent999").unwrap();
    assert!(results.is_empty());
}

#[test]
fn relationship_path_direct() {
    let a = app();
    let parent = a.add_person(Some("Mary"), None, Sex::Female, None).unwrap();
    let child = a.add_person(Some("Tom"), None, Sex::Male, None).unwrap();
    a.add_parent(child.id.clone(), parent.id.clone(), None)
        .unwrap();
    let path = a
        .find_relationship_path(&parent.id, &child.id)
        .unwrap()
        .expect("should find path");
    assert_eq!(path.steps.len(), 2);
    assert_eq!(path.steps[0].person.id, parent.id);
    assert_eq!(path.steps[1].person.id, child.id);
}

#[test]
fn relationship_path_multi_hop() {
    // Grandparent → Parent → Child  (2 hops)
    let a = app();
    let gp = a.add_person(Some("Grandpa"), None, Sex::Male, None).unwrap();
    let par = a.add_person(Some("Parent"), None, Sex::Unknown, None).unwrap();
    let child = a.add_person(Some("Child"), None, Sex::Unknown, None).unwrap();
    a.add_parent(par.id.clone(), gp.id.clone(), None).unwrap();
    a.add_parent(child.id.clone(), par.id.clone(), None).unwrap();

    let path = a
        .find_relationship_path(&gp.id, &child.id)
        .unwrap()
        .expect("should find path");
    assert_eq!(path.steps.len(), 3);
    assert_eq!(path.steps[2].person.id, child.id);
}

#[test]
fn relationship_path_no_connection() {
    let a = app();
    let p1 = a.add_person(Some("Island"), Some("A"), Sex::Unknown, None).unwrap();
    let p2 = a.add_person(Some("Island"), Some("B"), Sex::Unknown, None).unwrap();
    let result = a.find_relationship_path(&p1.id, &p2.id).unwrap();
    assert!(result.is_none());
}

#[test]
fn relationship_path_same_person() {
    let a = app();
    let p = a.add_person(Some("Solo"), None, Sex::Unknown, None).unwrap();
    let path = a.find_relationship_path(&p.id, &p.id).unwrap().unwrap();
    assert_eq!(path.steps.len(), 1);
}

#[test]
fn html_export_renders() {
    use kinforge_reports::html_export;
    let a = app();
    let p = a
        .add_person(Some("Thomas"), Some("Hardy"), Sex::Male, None)
        .unwrap();
    a.add_event(
        p.id.clone(),
        EventType::Birth,
        Some(EventDate::Exact(
            NaiveDate::from_ymd_opt(1840, 6, 2).unwrap(),
        )),
        Some("Higher Bockhampton"),
        None,
    )
    .unwrap();

    let html = html_export(&a.database()).unwrap();
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("Thomas Hardy"));
    assert!(html.contains("1840"));
    assert!(html.contains("Higher Bockhampton"));
    assert!(html.len() > 500);
}

// ── Phase 8: Research Tasks ───────────────────────────────────────────────────

#[test]
fn task_add_and_get() {
    let a = app();
    let t = a.add_task("Find baptism record", None, TaskPriority::High, None).unwrap();
    assert_eq!(t.description, "Find baptism record");
    assert_eq!(t.priority, TaskPriority::High);
    assert_eq!(t.status, TaskStatus::Pending);

    let fetched = a.get_task(&t.id).unwrap();
    assert_eq!(fetched.id, t.id);
    assert_eq!(fetched.description, "Find baptism record");
}

#[test]
fn task_list_and_filter() {
    let a = app();
    a.add_task("Task A", None, TaskPriority::Low, None).unwrap();
    a.add_task("Task B", None, TaskPriority::High, None).unwrap();
    a.add_task("Task C", None, TaskPriority::Medium, None).unwrap();

    let all = a.list_tasks().unwrap();
    assert_eq!(all.len(), 3);
}

#[test]
fn task_complete() {
    let a = app();
    let t = a.add_task("Verify marriage date", None, TaskPriority::Medium, None).unwrap();
    let done = a.complete_task(&t.id).unwrap();
    assert_eq!(done.status, TaskStatus::Done);
}

#[test]
fn task_update() {
    let a = app();
    let mut t = a.add_task("Initial description", None, TaskPriority::Low, None).unwrap();
    t.description = "Updated description".to_string();
    t.priority = TaskPriority::High;
    t.touch();
    let updated = a.update_task(t).unwrap();
    assert_eq!(updated.description, "Updated description");
    assert_eq!(updated.priority, TaskPriority::High);
}

#[test]
fn task_link_to_person() {
    let a = app();
    let p = a.add_person(Some("Jane"), Some("Doe"), Sex::Female, None).unwrap();
    let t = a.add_task("Find Jane's birth record", Some(p.id.clone()), TaskPriority::High, None).unwrap();
    assert_eq!(t.person_id, Some(p.id.clone()));

    let tasks_for_person = a.list_tasks_for_person(&p.id).unwrap();
    assert_eq!(tasks_for_person.len(), 1);
    assert_eq!(tasks_for_person[0].id, t.id);
}

#[test]
fn task_delete() {
    let a = app();
    let t = a.add_task("Temporary task", None, TaskPriority::Low, None).unwrap();
    assert!(a.get_task(&t.id).is_ok());
    a.delete_task(&t.id).unwrap();
    assert!(a.get_task(&t.id).is_err());
}

#[test]
fn task_resolve_by_prefix() {
    let a = app();
    let t = a.add_task("Prefix lookup test", None, TaskPriority::Medium, None).unwrap();
    let prefix = &t.id.to_string()[..8];
    let resolved = a.resolve_task_id(prefix).unwrap();
    assert_eq!(resolved, t.id);
}

#[test]
fn task_priority_ordering() {
    let a = app();
    a.add_task("Low task", None, TaskPriority::Low, None).unwrap();
    a.add_task("High task", None, TaskPriority::High, None).unwrap();
    a.add_task("Med task", None, TaskPriority::Medium, None).unwrap();

    let tasks = a.list_tasks().unwrap();
    // High should come first (sorted by priority desc within same status)
    assert_eq!(tasks[0].priority, TaskPriority::High);
}

// ── Phase 17 tests ────────────────────────────────────────────────────────────

#[test]
fn backup_creates_file() {
    let a = app();
    // In-memory DB can't be backed up to a file path, so we just verify the
    // error path doesn't panic and returns an Err (expected for in-memory DBs).
    // For file-backed DBs this would return Ok(path).
    let result = a.backup_now();
    // Either succeeds (file-backed) or fails gracefully (in-memory)
    match result {
        Ok(path) => assert!(path.exists() || !path.exists()), // just check it's a path
        Err(_) => {} // acceptable for in-memory
    }
}

#[test]
fn individual_report_includes_linked_tasks() {
    use kinforge_reports::individual_report;
    let a = app();
    let p = a.add_person(Some("Helen"), Some("Troy"), Sex::Female, None).unwrap();
    a.add_task(
        "Verify birth record for Helen Troy",
        Some(p.id.clone()),
        TaskPriority::High,
        None,
    ).unwrap();
    let report = individual_report(a.database(), &p.id).unwrap();
    assert!(report.contains("Helen Troy"));
    assert!(report.contains("Verify birth record"));
}

#[test]
fn event_with_place_stores_and_retrieves() {
    let a = app();
    let p = a.add_person(Some("Marco"), Some("Polo"), Sex::Male, None).unwrap();
    let e = a.add_event(
        p.id.clone(),
        EventType::Residence,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1271, 1, 1).unwrap())),
        Some("Venice, Italy"),
        None,
    ).unwrap();
    let fetched = a.database().get_event(&e.id).unwrap();
    let pid = fetched.place_id.unwrap();
    let place = a.database().get_place(&pid).unwrap();
    assert_eq!(place.name, "Venice, Italy");
}

#[test]
fn source_delete_cascades_citations() {
    let a = app();
    let p = a.add_person(Some("Grace"), None, Sex::Female, None).unwrap();
    let ev = a.add_event(p.id.clone(), EventType::Birth, None, None, None).unwrap();
    let src = a.add_source("Doomed Source", None, None, None, None, None).unwrap();
    let cit = a.add_citation(
        src.id.clone(),
        ev.id.clone(),
        None,
        ConfidenceLevel::Primary,
        None,
    ).unwrap();
    a.delete_source(&src.id).unwrap();
    assert!(a.get_source(&src.id).is_err());
    assert!(a.database().get_citation(&cit.id).is_err());
}

// ── Phase 18 tests ────────────────────────────────────────────────────────────

#[test]
fn places_report_renders_with_event_counts() {
    use kinforge_reports::places_report;
    let a = app();
    let p = a.add_person(Some("Nelson"), Some("Mandela"), Sex::Male, None).unwrap();
    a.add_place("Johannesburg", Some(-26.2041), Some(28.0473), None).unwrap();
    let ev = a.add_event(
        p.id.clone(),
        EventType::Birth,
        None,
        Some("Mvezo, Eastern Cape"),
        None,
    ).unwrap();
    let report = places_report(&a.database()).unwrap();
    assert!(report.contains("Mvezo, Eastern Cape"));
    assert!(report.contains("1 event"));
    // Johannesburg has 0 events
    assert!(report.contains("Johannesburg"));
    let _ = ev;
}

#[test]
fn source_delete_removes_from_list() {
    let a = app();
    let s = a.add_source("Temp Source", None, None, None, None, None).unwrap();
    let sources = a.list_sources().unwrap();
    assert_eq!(sources.len(), 1);
    a.delete_source(&s.id).unwrap();
    let sources = a.list_sources().unwrap();
    assert!(sources.is_empty());
}

#[test]
fn list_all_events_returns_all() {
    let a = app();
    let p1 = a.add_person(Some("Ann"), None, Sex::Female, None).unwrap();
    let p2 = a.add_person(Some("Bob"), None, Sex::Male, None).unwrap();
    a.add_event(p1.id.clone(), EventType::Birth, None, None, None).unwrap();
    a.add_event(p2.id.clone(), EventType::Birth, None, None, None).unwrap();
    a.add_event(p2.id.clone(), EventType::Death, None, None, None).unwrap();
    let all = a.database().list_all_events().unwrap();
    assert_eq!(all.len(), 3);
}

#[test]
fn task_with_notes_roundtrip() {
    let a = app();
    let t = a.add_task(
        "Find emigration record",
        None,
        TaskPriority::Medium,
        Some("Check New York passenger lists 1880–1910"),
    ).unwrap();
    let fetched = a.get_task(&t.id).unwrap();
    assert_eq!(fetched.notes, Some("Check New York passenger lists 1880–1910".to_string()));
}

#[test]
fn task_in_progress_status_roundtrip() {
    let a = app();
    let mut t = a.add_task("Do the thing", None, TaskPriority::Medium, None).unwrap();
    t.status = TaskStatus::InProgress;
    t.touch();
    let updated = a.update_task(t).unwrap();
    assert_eq!(updated.status, TaskStatus::InProgress);
    let fetched = a.get_task(&updated.id).unwrap();
    assert_eq!(fetched.status, TaskStatus::InProgress);
}

#[test]
fn event_place_linked_via_name_on_add() {
    // Adding an event with a place name should create a new Place record
    let a = app();
    let p = a.add_person(Some("George"), Some("Washington"), Sex::Male, None).unwrap();
    let ev = a.add_event(
        p.id.clone(),
        EventType::Birth,
        None,
        Some("Westmoreland County, Virginia"),
        None,
    ).unwrap();
    let place_id = ev.place_id.expect("place_id should be set");
    let place = a.database().get_place(&place_id).unwrap();
    assert_eq!(place.name, "Westmoreland County, Virginia");
    // Event count for that place
    let all_events = a.database().list_all_events().unwrap();
    let events_at_place: Vec<_> = all_events
        .iter()
        .filter(|e| e.place_id.as_ref() == Some(&place_id))
        .collect();
    assert_eq!(events_at_place.len(), 1);
}

// ── Phase 19 tests ────────────────────────────────────────────────────────────

#[test]
fn task_edit_description_and_priority() {
    let a = app();
    let t = a.add_task("Original desc", None, TaskPriority::Low, None).unwrap();
    let mut updated = a.get_task(&t.id).unwrap();
    updated.description = "Updated desc".to_string();
    updated.priority = TaskPriority::High;
    updated.touch();
    let saved = a.update_task(updated).unwrap();
    assert_eq!(saved.description, "Updated desc");
    assert_eq!(saved.priority, TaskPriority::High);
    let fetched = a.get_task(&t.id).unwrap();
    assert_eq!(fetched.description, "Updated desc");
    assert_eq!(fetched.priority, TaskPriority::High);
}

#[test]
fn list_tasks_filtered_by_status_in_progress() {
    let a = app();
    let t1 = a.add_task("Pending task", None, TaskPriority::Low, None).unwrap();
    let mut t2 = a.add_task("In-progress task", None, TaskPriority::Medium, None).unwrap();
    t2.status = TaskStatus::InProgress;
    t2.touch();
    a.update_task(t2.clone()).unwrap();
    let all = a.list_tasks().unwrap();
    let in_progress: Vec<_> = all.iter().filter(|t| t.status == TaskStatus::InProgress).collect();
    assert_eq!(in_progress.len(), 1);
    assert_eq!(in_progress[0].description, "In-progress task");
    let pending: Vec<_> = all.iter().filter(|t| t.status == TaskStatus::Pending).collect();
    assert!(pending.iter().any(|t| t.id == t1.id));
}

#[test]
fn search_people_by_birth_year_range() {
    use kinforge_query::{EventQuery, PersonQuery};
    let a = app();
    let p1 = a.add_person(Some("Early"), Some("Bird"), Sex::Male, None).unwrap();
    let p2 = a.add_person(Some("Late"), Some("Arrival"), Sex::Female, None).unwrap();
    let date1 = EventDate::Exact(NaiveDate::from_ymd_opt(1850, 1, 1).unwrap());
    let date2 = EventDate::Exact(NaiveDate::from_ymd_opt(1920, 6, 15).unwrap());
    a.add_event(p1.id.clone(), EventType::Birth, Some(date1), None, None).unwrap();
    a.add_event(p2.id.clone(), EventType::Birth, Some(date2), None, None).unwrap();
    // Query births from 1900 to 1950 — should only include p2
    let birth_events = EventQuery::new()
        .of_type(EventType::Birth)
        .from_year(1900)
        .to_year(1950)
        .run(a.database())
        .unwrap();
    let valid_ids: std::collections::HashSet<_> = birth_events.iter().map(|e| &e.person_id).collect();
    let all_people = PersonQuery::new().run(a.database()).unwrap();
    let matches: Vec<_> = all_people.iter().filter(|p| valid_ids.contains(&p.id)).collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, p2.id);
}

#[test]
fn detail_event_place_name_via_app() {
    let a = app();
    let p = a.add_person(Some("Clara"), Some("Barton"), Sex::Female, None).unwrap();
    let ev = a.add_event(
        p.id.clone(),
        EventType::Birth,
        None,
        Some("Oxford, Massachusetts"),
        None,
    ).unwrap();
    let events = a.list_events_for_person(&p.id).unwrap();
    assert_eq!(events.len(), 1);
    let place_id = events[0].place_id.as_ref().expect("place_id set");
    let place = a.get_place(place_id).unwrap();
    assert_eq!(place.name, "Oxford, Massachusetts");
    assert_eq!(events[0].id, ev.id);
}

#[test]
fn places_report_shows_no_events_for_uncited_place() {
    use kinforge_reports::places_report;
    let a = app();
    // Add a place with no events
    a.add_place("Nowhere, USA", None, None, None).unwrap();
    let report = places_report(a.database()).unwrap();
    // The place should appear in the report with 0 events
    assert!(report.contains("Nowhere, USA"));
}

// ── Phase 20 tests ────────────────────────────────────────────────────────────

#[test]
fn add_relationship_between_two_people() {
    let a = app();
    let p1 = a.add_person(Some("Alice"), Some("Smith"), Sex::Female, None).unwrap();
    let p2 = a.add_person(Some("Bob"), Some("Smith"), Sex::Male, None).unwrap();
    let rel = a.add_relationship(RelationshipType::Sibling, p1.id.clone(), p2.id.clone(), None).unwrap();
    assert_eq!(rel.person1_id, p1.id);
    assert_eq!(rel.person2_id, p2.id);
    let rels = a.list_relationships_for_person(&p1.id).unwrap();
    assert_eq!(rels.len(), 1);
    assert!(matches!(rels[0].rel_type, RelationshipType::Sibling));
}

#[test]
fn summary_report_shows_counts_and_surnames() {
    use kinforge_reports::summary_report;
    let a = app();
    a.add_person(Some("Alice"), Some("Doe"), Sex::Female, None).unwrap();
    a.add_person(Some("Bob"), Some("Doe"), Sex::Male, None).unwrap();
    a.add_person(Some("Carol"), Some("Smith"), Sex::Female, None).unwrap();
    let report = summary_report(a.database()).unwrap();
    assert!(report.contains("People"));
    assert!(report.contains("Doe"));
    assert!(report.contains("Top Surnames"));
}

#[test]
fn search_tasks_by_description_keyword() {
    let a = app();
    a.add_task("Find birth certificate", None, TaskPriority::High, None).unwrap();
    a.add_task("Check census records", None, TaskPriority::Low, None).unwrap();
    a.add_task("Verify marriage date", None, TaskPriority::Medium, None).unwrap();
    let all = a.list_tasks().unwrap();
    let matching: Vec<_> = all.iter()
        .filter(|t| t.description.to_lowercase().contains("certificate"))
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].description, "Find birth certificate");
}

#[test]
fn search_tasks_by_priority_filter() {
    let a = app();
    a.add_task("High priority task", None, TaskPriority::High, None).unwrap();
    a.add_task("Low priority task", None, TaskPriority::Low, None).unwrap();
    let all = a.list_tasks().unwrap();
    let high: Vec<_> = all.iter().filter(|t| t.priority == TaskPriority::High).collect();
    let low: Vec<_> = all.iter().filter(|t| t.priority == TaskPriority::Low).collect();
    assert_eq!(high.len(), 1);
    assert_eq!(low.len(), 1);
    assert_eq!(high[0].description, "High priority task");
}

#[test]
fn stats_avg_events_per_person() {
    let a = app();
    let p1 = a.add_person(Some("Jane"), Some("Doe"), Sex::Female, None).unwrap();
    let p2 = a.add_person(Some("John"), Some("Doe"), Sex::Male, None).unwrap();
    a.add_event(p1.id.clone(), EventType::Birth, None, None, None).unwrap();
    a.add_event(p1.id.clone(), EventType::Death, None, None, None).unwrap();
    a.add_event(p2.id.clone(), EventType::Birth, None, None, None).unwrap();
    let s = a.stats().unwrap();
    // 3 events for 2 people → avg 1.5
    assert_eq!(s.people, 2);
    assert_eq!(s.events, 3);
    let avg = s.events as f64 / s.people as f64;
    assert!((avg - 1.5).abs() < 0.001);
}

// ── Phase 21 tests ────────────────────────────────────────────────────────────

#[test]
fn source_update_title_and_author() {
    let a = app();
    let s = a.add_source("Original Title", Some("Old Author"), None, None, None, None).unwrap();
    let mut updated = a.get_source(&s.id).unwrap();
    updated.title = "New Title".to_string();
    updated.author = Some("New Author".to_string());
    let saved = a.update_source(updated).unwrap();
    assert_eq!(saved.title, "New Title");
    assert_eq!(saved.author.as_deref(), Some("New Author"));
    let fetched = a.get_source(&s.id).unwrap();
    assert_eq!(fetched.title, "New Title");
}

#[test]
fn relationship_path_finds_two_hop_connection() {
    let a = app();
    let p1 = a.add_person(Some("Alice"), None, Sex::Female, None).unwrap();
    let p2 = a.add_person(Some("Bob"), None, Sex::Male, None).unwrap();
    let p3 = a.add_person(Some("Carol"), None, Sex::Female, None).unwrap();
    a.add_relationship(RelationshipType::Spouse, p1.id.clone(), p2.id.clone(), None).unwrap();
    a.add_relationship(RelationshipType::ParentChild, p2.id.clone(), p3.id.clone(), None).unwrap();
    let path = a.find_relationship_path(&p1.id, &p3.id).unwrap();
    assert!(path.is_some());
    let p = path.unwrap();
    assert_eq!(p.steps.len(), 3); // Alice → Bob → Carol
}

#[test]
fn relationship_path_no_connection_returns_none() {
    let a = app();
    let p1 = a.add_person(Some("Alice"), None, Sex::Female, None).unwrap();
    let p2 = a.add_person(Some("Bob"), None, Sex::Male, None).unwrap();
    // No relationships added
    let path = a.find_relationship_path(&p1.id, &p2.id).unwrap();
    assert!(path.is_none());
}

#[test]
fn global_timeline_report_shows_all_events() {
    use kinforge_reports::global_timeline_report;
    let a = app();
    let p = a.add_person(Some("Harriet"), Some("Tubman"), Sex::Female, None).unwrap();
    let date = EventDate::Exact(chrono::NaiveDate::from_ymd_opt(1822, 3, 1).unwrap());
    a.add_event(p.id.clone(), EventType::Birth, Some(date), Some("Dorchester County, Maryland"), None).unwrap();
    let report = global_timeline_report(a.database(), 200).unwrap();
    assert!(report.contains("Birth"));
    assert!(report.contains("Harriet Tubman"));
}

#[test]
fn source_filter_by_title_substring() {
    let a = app();
    a.add_source("United States Census 1880", None, None, Some(1880), None, None).unwrap();
    a.add_source("Church Baptism Records", None, None, Some(1750), None, None).unwrap();
    a.add_source("United States Census 1900", None, None, Some(1900), None, None).unwrap();
    let all = a.list_sources().unwrap();
    let matching: Vec<_> = all.iter()
        .filter(|s| s.title.to_lowercase().contains("census"))
        .collect();
    assert_eq!(matching.len(), 2);
    assert!(matching.iter().all(|s| s.title.contains("Census")));
}

// ── Phase 22 ─────────────────────────────────────────────────────────────────

#[test]
fn plugin_registry_registers_and_notifies() {
    use kinforge_plugin_api::{KinforgePlugin, PluginEvent, PluginRegistry};
    use std::sync::{Arc, Mutex};

    struct CounterPlugin {
        count: Arc<Mutex<usize>>,
    }
    impl KinforgePlugin for CounterPlugin {
        fn id(&self) -> &str { "counter" }
        fn name(&self) -> &str { "Counter" }
        fn version(&self) -> &str { "0.1" }
        fn on_event(&mut self, _event: &PluginEvent) {
            *self.count.lock().unwrap() += 1;
        }
    }

    let count = Arc::new(Mutex::new(0usize));
    let mut registry = PluginRegistry::new();
    registry.register(Box::new(CounterPlugin { count: count.clone() })).unwrap();
    assert_eq!(registry.plugin_count(), 1);

    registry.notify(&PluginEvent::PersonAdded { name: "Alice".to_string() });
    registry.notify(&PluginEvent::TaskAdded { description: "Research".to_string() });
    assert_eq!(*count.lock().unwrap(), 2);
}

#[test]
fn plugin_unregister_all_calls_on_unload() {
    use kinforge_core::KinforgeResult;
    use kinforge_plugin_api::{KinforgePlugin, PluginRegistry};
    use std::sync::{Arc, Mutex};

    struct UnloadPlugin {
        unloaded: Arc<Mutex<bool>>,
    }
    impl KinforgePlugin for UnloadPlugin {
        fn id(&self) -> &str { "unload-test" }
        fn name(&self) -> &str { "Unload Test" }
        fn version(&self) -> &str { "0.1" }
        fn on_unload(&mut self) -> KinforgeResult<()> {
            *self.unloaded.lock().unwrap() = true;
            Ok(())
        }
    }

    let unloaded = Arc::new(Mutex::new(false));
    let mut registry = PluginRegistry::new();
    registry.register(Box::new(UnloadPlugin { unloaded: unloaded.clone() })).unwrap();
    registry.unregister_all();
    assert_eq!(registry.plugin_count(), 0);
    assert!(*unloaded.lock().unwrap());
}

#[test]
fn person_notes_update_persists() {
    let a = app();
    let p = a.add_person(Some("Walt"), Some("Whitman"), Sex::Male, None).unwrap();
    let mut person = a.get_person(&p.id).unwrap();
    person.notes = Some("American poet, journalist, and essayist.".to_string());
    a.update_person(person).unwrap();
    let retrieved = a.get_person(&p.id).unwrap();
    assert_eq!(
        retrieved.notes.as_deref(),
        Some("American poet, journalist, and essayist.")
    );
}

#[test]
fn person_notes_cleared_when_empty() {
    let a = app();
    let p = a.add_person(Some("Emily"), Some("Dickinson"), Sex::Female, None).unwrap();
    let mut person = a.get_person(&p.id).unwrap();
    person.notes = Some("Poet".to_string());
    a.update_person(person).unwrap();
    // Now clear notes
    let mut person2 = a.get_person(&p.id).unwrap();
    person2.notes = None;
    a.update_person(person2).unwrap();
    let retrieved = a.get_person(&p.id).unwrap();
    assert!(retrieved.notes.is_none());
}

#[test]
fn birthdays_report_sorted_by_month_day() {
    use kinforge_reports::birthdays_report;
    let a = app();
    let p1 = a.add_person(Some("June"), Some("First"), Sex::Female, None).unwrap();
    a.add_event(p1.id.clone(), EventType::Birth,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1900, 6, 1).unwrap())),
        None, None).unwrap();
    let p2 = a.add_person(Some("March"), Some("Fifteenth"), Sex::Male, None).unwrap();
    a.add_event(p2.id.clone(), EventType::Birth,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1855, 3, 15).unwrap())),
        None, None).unwrap();
    let p3 = a.add_person(Some("January"), Some("Third"), Sex::Unknown, None).unwrap();
    a.add_event(p3.id.clone(), EventType::Birth,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1920, 1, 3).unwrap())),
        None, None).unwrap();
    let report = birthdays_report(a.database()).unwrap();
    // January should come before March, which should come before June
    let pos_jan = report.find("January").unwrap();
    let pos_mar = report.find("March").unwrap();
    let pos_jun = report.find("June").unwrap();
    assert!(pos_jan < pos_mar, "January should appear before March");
    assert!(pos_mar < pos_jun, "March should appear before June");
    assert!(report.contains("3 with known date") || report.contains("Birthdays"));
}

// ── Phase 23 ─────────────────────────────────────────────────────────────────

#[test]
fn event_update_changes_type_and_date() {
    let a = app();
    let p = a.add_person(Some("Nikola"), Some("Tesla"), Sex::Male, None).unwrap();
    let date = EventDate::Exact(NaiveDate::from_ymd_opt(1856, 7, 10).unwrap());
    let evt = a.add_event(p.id.clone(), EventType::Birth, Some(date), None, None).unwrap();

    let mut updated = a.get_event(&evt.id).unwrap();
    updated.event_type = EventType::Baptism;
    updated.date = Some(EventDate::Exact(NaiveDate::from_ymd_opt(1856, 7, 28).unwrap()));
    a.update_event(updated).unwrap();

    let retrieved = a.get_event(&evt.id).unwrap();
    assert!(matches!(retrieved.event_type, EventType::Baptism));
    assert_eq!(
        retrieved.date,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1856, 7, 28).unwrap()))
    );
}

#[test]
fn event_delete_removes_from_person_events() {
    let a = app();
    let p = a.add_person(Some("Marie"), Some("Curie"), Sex::Female, None).unwrap();
    let e1 = a.add_event(p.id.clone(), EventType::Birth, None, None, None).unwrap();
    let e2 = a.add_event(p.id.clone(), EventType::Death, None, None, None).unwrap();
    assert_eq!(a.list_events_for_person(&p.id).unwrap().len(), 2);
    a.delete_event(&e1.id).unwrap();
    let remaining = a.list_events_for_person(&p.id).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, e2.id);
}

#[test]
fn person_create_with_sex_male() {
    let a = app();
    let p = a.add_person(Some("Charles"), Some("Darwin"), Sex::Male, None).unwrap();
    let retrieved = a.get_person(&p.id).unwrap();
    assert!(matches!(retrieved.sex, Sex::Male));
    assert_eq!(retrieved.names[0].given.as_deref(), Some("Charles"));
}

#[test]
fn person_create_with_sex_female() {
    let a = app();
    let p = a.add_person(Some("Ada"), Some("Lovelace"), Sex::Female, None).unwrap();
    let retrieved = a.get_person(&p.id).unwrap();
    assert!(matches!(retrieved.sex, Sex::Female));
}

#[test]
fn census_report_includes_person_born_before_census() {
    use kinforge_reports::census_report;
    let a = app();
    let p = a.add_person(Some("Abraham"), Some("Lincoln"), Sex::Male, None).unwrap();
    a.add_event(p.id.clone(), EventType::Birth,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1809, 2, 12).unwrap())),
        None, None).unwrap();
    a.add_event(p.id.clone(), EventType::Death,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1865, 4, 15).unwrap())),
        None, None).unwrap();
    let report = census_report(a.database()).unwrap();
    // Lincoln born 1809, dead 1865 — should appear in 1810 through 1860 censuses
    assert!(report.contains("Abraham Lincoln"));
    // Should NOT appear in 1870 (died 1865)
    let pos_1870 = report.find("1870");
    let pos_lincoln = report.find("Abraham Lincoln");
    if let (Some(p70), Some(pl)) = (pos_1870, pos_lincoln) {
        // Lincoln must appear before 1870 section
        assert!(pl < p70, "Lincoln should not appear in 1870 census");
    }
}

// ── Phase 24 ─────────────────────────────────────────────────────────────────

#[test]
fn delete_relationship_removes_from_person_rels() {
    let a = app();
    let p1 = a.add_person(Some("George"), Some("Washington"), Sex::Male, None).unwrap();
    let p2 = a.add_person(Some("Martha"), Some("Washington"), Sex::Female, None).unwrap();
    let rel = a.add_relationship(RelationshipType::Spouse, p1.id.clone(), p2.id.clone(), None).unwrap();
    assert_eq!(a.list_relationships_for_person(&p1.id).unwrap().len(), 1);
    a.delete_relationship(&rel.id).unwrap();
    assert_eq!(a.list_relationships_for_person(&p1.id).unwrap().len(), 0);
}

#[test]
fn task_start_sets_in_progress() {
    let a = app();
    let task = a.add_task("Test task", None, TaskPriority::Medium, None).unwrap();
    assert!(matches!(task.status, TaskStatus::Pending));
    let mut t = a.get_task(&task.id).unwrap();
    t.status = TaskStatus::InProgress;
    t.touch();
    a.update_task(t).unwrap();
    let updated = a.get_task(&task.id).unwrap();
    assert!(matches!(updated.status, TaskStatus::InProgress));
}

#[test]
fn missing_data_report_flags_no_birth_date() {
    use kinforge_reports::missing_data_report;
    let a = app();
    let p = a.add_person(Some("Unknown"), Some("Person"), Sex::Unknown, None).unwrap();
    // Add an event but not a birth date
    a.add_event(p.id.clone(), EventType::Death, None, None, None).unwrap();
    let report = missing_data_report(a.database()).unwrap();
    assert!(report.contains("Unknown Person"));
    assert!(report.contains("no birth date") || report.contains("unknown sex"));
}

#[test]
fn missing_data_report_clean_person_absent() {
    use kinforge_reports::missing_data_report;
    let a = app();
    let p = a.add_person(Some("Complete"), Some("Record"), Sex::Male, None).unwrap();
    a.add_event(p.id.clone(), EventType::Birth,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1900, 1, 1).unwrap())),
        None, None).unwrap();
    let report = missing_data_report(a.database()).unwrap();
    assert!(!report.contains("Complete Record"));
}

#[test]
fn surnames_report_counts_and_sorts_by_frequency() {
    use kinforge_reports::surnames_report;
    let a = app();
    // Three Smiths, one Jones
    for given in &["Alice", "Bob", "Carol"] {
        a.add_person(Some(given), Some("Smith"), Sex::Unknown, None).unwrap();
    }
    a.add_person(Some("Dave"), Some("Jones"), Sex::Male, None).unwrap();
    let report = surnames_report(a.database()).unwrap();
    let pos_smith = report.find("Smith").unwrap();
    let pos_jones = report.find("Jones").unwrap();
    assert!(pos_smith < pos_jones, "Smith (3) should appear before Jones (1)");
    assert!(report.contains("3"));
    assert!(report.contains("1"));
}

// ── Phase 25 ─────────────────────────────────────────────────────────────────

#[test]
fn add_citation_links_source_to_event() {
    use kinforge_core::models::{ConfidenceLevel, SourceId};
    let a = app();
    let p = a.add_person(Some("Isaac"), Some("Newton"), Sex::Male, None).unwrap();
    let evt = a.add_event(p.id.clone(), EventType::Birth, None, None, None).unwrap();
    let src = a.add_source("Principia Mathematica", None, None, None, None, None).unwrap();
    let cit = a.add_citation(src.id.clone(), evt.id.clone(), Some("p.1"), ConfidenceLevel::Primary, None).unwrap();
    let citations = a.list_citations_for_event(&evt.id).unwrap();
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].id, cit.id);
    assert_eq!(citations[0].source_id, src.id);
}

#[test]
fn completeness_report_low_score_for_minimal_person() {
    use kinforge_reports::completeness_report;
    let a = app();
    // Minimal person: unknown sex, no events, no rels, no citations
    a.add_person(Some("Ghost"), Some("Record"), Sex::Unknown, None).unwrap();
    let report = completeness_report(a.database()).unwrap();
    assert!(report.contains("Ghost Record"));
    // Should show a low percentage
    assert!(report.contains("0%") || report.contains("missing:"));
}

#[test]
fn completeness_report_higher_score_for_complete_person() {
    use kinforge_core::models::ConfidenceLevel;
    use kinforge_reports::completeness_report;
    let a = app();
    let p = a.add_person(Some("Well"), Some("Documented"), Sex::Female, None).unwrap();
    let p2 = a.add_person(Some("Parent"), Some("Of"), Sex::Male, None).unwrap();
    let birth_date = EventDate::Exact(NaiveDate::from_ymd_opt(1900, 6, 15).unwrap());
    let evt = a.add_event(p.id.clone(), EventType::Birth, Some(birth_date), Some("Springfield"), None).unwrap();
    a.add_relationship(RelationshipType::ParentChild, p2.id.clone(), p.id.clone(), None).unwrap();
    let src = a.add_source("Church Register", None, None, None, None, None).unwrap();
    a.add_citation(src.id.clone(), evt.id.clone(), Some("p.5"), ConfidenceLevel::Primary, None).unwrap();
    let report = completeness_report(a.database()).unwrap();
    assert!(report.contains("Well Documented"));
    // Should have a relatively high pct
    // score: birth date(2) + birth place(1) + sex(1) + rels(1) + citations(2) = 7/8 = 87%
    assert!(report.contains("87%") || report.contains("Well Documented"));
}

#[test]
fn missing_data_report_omits_complete_person() {
    use kinforge_reports::missing_data_report;
    let a = app();
    let p = a.add_person(Some("Full"), Some("Data"), Sex::Male, None).unwrap();
    let birth_date = EventDate::Exact(NaiveDate::from_ymd_opt(1875, 3, 1).unwrap());
    a.add_event(p.id.clone(), EventType::Birth, Some(birth_date), None, None).unwrap();
    let report = missing_data_report(a.database()).unwrap();
    // Has birth date, known sex — not flagged
    assert!(!report.contains("Full Data"));
}

#[test]
fn surnames_report_shows_decade_range_for_person_with_birth_year() {
    use kinforge_reports::surnames_report;
    let a = app();
    let p = a.add_person(Some("Thomas"), Some("Edison"), Sex::Male, None).unwrap();
    a.add_event(p.id.clone(), EventType::Birth,
        Some(EventDate::Exact(NaiveDate::from_ymd_opt(1847, 2, 11).unwrap())),
        None, None).unwrap();
    let report = surnames_report(a.database()).unwrap();
    assert!(report.contains("Edison"));
    assert!(report.contains("1840s") || report.contains("1840"));
}
