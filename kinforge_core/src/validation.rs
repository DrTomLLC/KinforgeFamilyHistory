use crate::{models::*, KinforgeError, KinforgeResult};

/// Validate a `Person` before storage.
pub fn validate_person(person: &Person) -> KinforgeResult<()> {
    // Must have at least one name if names are provided
    for name in &person.names {
        if name.given.is_none() && name.surname.is_none() {
            return Err(KinforgeError::Validation(
                "A PersonName must have at least a given name or surname".to_string(),
            ));
        }
    }
    Ok(())
}

/// Validate an `Event` before storage.
pub fn validate_event(event: &Event) -> KinforgeResult<()> {
    // EventDate::Between must have d1 ≤ d2
    if let Some(EventDate::Between(d1, d2)) = &event.date {
        if d1 > d2 {
            return Err(KinforgeError::Validation(
                "EventDate::Between start date must be ≤ end date".to_string(),
            ));
        }
    }
    Ok(())
}

/// Validate a `Relationship` before storage.
pub fn validate_relationship(rel: &Relationship) -> KinforgeResult<()> {
    if rel.person1_id == rel.person2_id {
        return Err(KinforgeError::Validation(
            "A relationship cannot link a person to themselves".to_string(),
        ));
    }
    Ok(())
}

/// Validate a `Source` before storage.
pub fn validate_source(source: &Source) -> KinforgeResult<()> {
    if source.title.trim().is_empty() {
        return Err(KinforgeError::Validation(
            "Source title must not be empty".to_string(),
        ));
    }
    if let Some(year) = source.year {
        if !(1..=2100).contains(&year) {
            return Err(KinforgeError::Validation(format!(
                "Source year {} is implausible",
                year
            )));
        }
    }
    Ok(())
}

/// Validate a `Place` before storage.
pub fn validate_place(place: &Place) -> KinforgeResult<()> {
    if place.name.trim().is_empty() {
        return Err(KinforgeError::Validation(
            "Place name must not be empty".to_string(),
        ));
    }
    if let Some(lat) = place.latitude {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(KinforgeError::Validation(format!(
                "Latitude {} is out of range [-90, 90]",
                lat
            )));
        }
    }
    if let Some(lon) = place.longitude {
        if !(-180.0..=180.0).contains(&lon) {
            return Err(KinforgeError::Validation(format!(
                "Longitude {} is out of range [-180, 180]",
                lon
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_name_invalid() {
        let mut p = Person::new(Sex::Unknown);
        p.names.push(PersonName {
            given: None,
            surname: None,
            name_type: NameType::Birth,
            prefix: None,
            suffix: None,
        });
        assert!(validate_person(&p).is_err());
    }

    #[test]
    fn test_valid_person() {
        let mut p = Person::new(Sex::Male);
        p.names.push(PersonName {
            given: Some("John".into()),
            surname: None,
            name_type: NameType::Birth,
            prefix: None,
            suffix: None,
        });
        assert!(validate_person(&p).is_ok());
    }

    #[test]
    fn test_self_relationship_invalid() {
        let id = PersonId::new();
        let rel = Relationship::new(RelationshipType::Spouse, id.clone(), id.clone());
        assert!(validate_relationship(&rel).is_err());
    }

    #[test]
    fn test_empty_source_title_invalid() {
        let source = Source::new("   ");
        assert!(validate_source(&source).is_err());
    }

    #[test]
    fn test_empty_place_name_invalid() {
        let mut place = Place::new("");
        assert!(validate_place(&place).is_err());
        place.latitude = Some(200.0);
        place.name = "Valid".to_string();
        assert!(validate_place(&place).is_err());
    }
}
