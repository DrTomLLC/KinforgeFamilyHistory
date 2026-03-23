use serde::{Deserialize, Serialize};

use super::ids::PersonId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonId,
    pub names: Vec<PersonName>,
    pub sex: Sex,
    pub notes: Option<String>,
}

impl Person {
    pub fn new(sex: Sex) -> Self {
        Self {
            id: PersonId::new(),
            names: Vec::new(),
            sex,
            notes: None,
        }
    }

    pub fn primary_name(&self) -> Option<&PersonName> {
        self.names.first()
    }

    pub fn display_name(&self) -> String {
        match self.primary_name() {
            Some(n) => n.full_name(),
            None => "(unnamed)".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonName {
    pub given: Option<String>,
    pub surname: Option<String>,
    pub name_type: NameType,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

impl PersonName {
    pub fn full_name(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref p) = self.prefix {
            parts.push(p.clone());
        }
        if let Some(ref g) = self.given {
            parts.push(g.clone());
        }
        if let Some(ref s) = self.surname {
            parts.push(s.clone());
        }
        if let Some(ref sfx) = self.suffix {
            parts.push(sfx.clone());
        }
        if parts.is_empty() {
            "(unnamed)".to_string()
        } else {
            parts.join(" ")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Sex {
    Male,
    Female,
    #[default]
    Unknown,
}

impl fmt::Display for Sex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sex::Male => write!(f, "Male"),
            Sex::Female => write!(f, "Female"),
            Sex::Unknown => write!(f, "Unknown"),
        }
    }
}

use std::fmt;

impl std::str::FromStr for Sex {
    type Err = crate::error::KinforgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "m" | "male" => Ok(Sex::Male),
            "f" | "female" => Ok(Sex::Female),
            "u" | "unknown" | "" => Ok(Sex::Unknown),
            _ => Err(crate::error::KinforgeError::InvalidField {
                field: "sex".to_string(),
                value: s.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NameType {
    #[default]
    Birth,
    Married,
    AlsoKnownAs,
    Nickname,
    Other,
}

impl fmt::Display for NameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameType::Birth => write!(f, "Birth"),
            NameType::Married => write!(f, "Married"),
            NameType::AlsoKnownAs => write!(f, "AlsoKnownAs"),
            NameType::Nickname => write!(f, "Nickname"),
            NameType::Other => write!(f, "Other"),
        }
    }
}

impl std::str::FromStr for NameType {
    type Err = crate::error::KinforgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "birth" => Ok(NameType::Birth),
            "married" | "marriage" => Ok(NameType::Married),
            "aka" | "alsoknownas" | "also_known_as" => Ok(NameType::AlsoKnownAs),
            "nickname" | "nick" => Ok(NameType::Nickname),
            "other" => Ok(NameType::Other),
            _ => Err(crate::error::KinforgeError::InvalidField {
                field: "name_type".to_string(),
                value: s.to_string(),
            }),
        }
    }
}
