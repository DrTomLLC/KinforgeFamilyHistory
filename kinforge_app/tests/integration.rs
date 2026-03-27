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
