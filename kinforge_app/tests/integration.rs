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
    assert!(a.db.get_event(&e.id).is_err());
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
    let fetched = a.db.get_event(&e.id).unwrap();
    match &fetched.date {
        Some(EventDate::Exact(d)) => assert_eq!(*d, NaiveDate::from_ymd_opt(1900, 1, 1).unwrap()),
        _ => panic!("wrong date kind"),
    }
    let place = a.db.get_place(fetched.place_id.as_ref().unwrap()).unwrap();
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
    let fetched = a.db.get_event(&e.id).unwrap();
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
    assert!(a.db.get_event(&e.id).is_err());
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
    assert!(a.db.get_relationship(&rel.id).is_err());
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
    let fetched = a.db.get_citation(&cit.id).unwrap();
    assert_eq!(fetched.page, Some("vol.3 p.99".to_string()));
    a.delete_citation(&cit.id).unwrap();
    assert!(a.db.get_citation(&cit.id).is_err());
}

// ── Place CRUD ────────────────────────────────────────────────────────────────

#[test]
fn place_crud() {
    let a = app();
    let pl = a
        .add_place("London, England", Some(51.5074), Some(-0.1278), None)
        .unwrap();
    let mut fetched = a.db.get_place(&pl.id).unwrap();
    assert_eq!(fetched.name, "London, England");
    fetched.name = "London, UK".to_string();
    a.update_place(fetched.clone()).unwrap();
    let updated = a.db.get_place(&pl.id).unwrap();
    assert_eq!(updated.name, "London, UK");
    a.delete_place(&pl.id).unwrap();
    assert!(a.db.get_place(&pl.id).is_err());
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
