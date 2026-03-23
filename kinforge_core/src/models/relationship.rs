use serde::{Deserialize, Serialize};
use std::fmt;

use super::ids::{PersonId, RelationshipId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    pub id: RelationshipId,
    pub rel_type: RelationshipType,
    pub person1_id: PersonId,
    pub person2_id: PersonId,
    pub notes: Option<String>,
}

impl Relationship {
    pub fn new(rel_type: RelationshipType, person1_id: PersonId, person2_id: PersonId) -> Self {
        Self {
            id: RelationshipId::new(),
            rel_type,
            person1_id,
            person2_id,
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    /// person1 is the parent of person2
    ParentChild,
    /// person1 and person2 are spouses
    Spouse,
    /// person1 and person2 are siblings
    Sibling,
}

impl fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationshipType::ParentChild => write!(f, "ParentChild"),
            RelationshipType::Spouse => write!(f, "Spouse"),
            RelationshipType::Sibling => write!(f, "Sibling"),
        }
    }
}

impl std::str::FromStr for RelationshipType {
    type Err = crate::error::KinforgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "parentchild" | "parent_child" | "parent-child" | "parent" => {
                Ok(RelationshipType::ParentChild)
            }
            "spouse" | "married" | "partner" => Ok(RelationshipType::Spouse),
            "sibling" | "brother" | "sister" => Ok(RelationshipType::Sibling),
            _ => Err(crate::error::KinforgeError::InvalidField {
                field: "relationship_type".to_string(),
                value: s.to_string(),
            }),
        }
    }
}
