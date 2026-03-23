use serde::{Deserialize, Serialize};
use std::fmt;

use super::{
    date::EventDate,
    ids::{EventId, PersonId, PlaceId},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub event_type: EventType,
    pub person_id: PersonId,
    pub date: Option<EventDate>,
    pub place_id: Option<PlaceId>,
    pub notes: Option<String>,
}

impl Event {
    pub fn new(event_type: EventType, person_id: PersonId) -> Self {
        Self {
            id: EventId::new(),
            event_type,
            person_id,
            date: None,
            place_id: None,
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    Birth,
    Death,
    Marriage,
    Divorce,
    Burial,
    Baptism,
    Residence,
    Occupation,
    Education,
    Military,
    Naturalization,
    Census,
    Emigration,
    Immigration,
    Other(String),
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::Birth => write!(f, "Birth"),
            EventType::Death => write!(f, "Death"),
            EventType::Marriage => write!(f, "Marriage"),
            EventType::Divorce => write!(f, "Divorce"),
            EventType::Burial => write!(f, "Burial"),
            EventType::Baptism => write!(f, "Baptism"),
            EventType::Residence => write!(f, "Residence"),
            EventType::Occupation => write!(f, "Occupation"),
            EventType::Education => write!(f, "Education"),
            EventType::Military => write!(f, "Military"),
            EventType::Naturalization => write!(f, "Naturalization"),
            EventType::Census => write!(f, "Census"),
            EventType::Emigration => write!(f, "Emigration"),
            EventType::Immigration => write!(f, "Immigration"),
            EventType::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::str::FromStr for EventType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "birth" | "birt" => EventType::Birth,
            "death" | "deat" => EventType::Death,
            "marriage" | "marr" => EventType::Marriage,
            "divorce" | "div" => EventType::Divorce,
            "burial" | "buri" => EventType::Burial,
            "baptism" | "bapt" | "chr" => EventType::Baptism,
            "residence" | "resi" => EventType::Residence,
            "occupation" | "occu" => EventType::Occupation,
            "education" | "educ" => EventType::Education,
            "military" | "mili" => EventType::Military,
            "naturalization" | "natu" => EventType::Naturalization,
            "census" | "cens" => EventType::Census,
            "emigration" | "emig" => EventType::Emigration,
            "immigration" | "immi" => EventType::Immigration,
            other => EventType::Other(other.to_string()),
        })
    }
}
