use serde::{Deserialize, Serialize};
use std::fmt;

use super::ids::{CitationId, EventId, SourceId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Citation {
    pub id: CitationId,
    pub source_id: SourceId,
    pub event_id: EventId,
    pub page: Option<String>,
    pub confidence: ConfidenceLevel,
    pub notes: Option<String>,
}

impl Citation {
    pub fn new(source_id: SourceId, event_id: EventId) -> Self {
        Self {
            id: CitationId::new(),
            source_id,
            event_id,
            page: None,
            confidence: ConfidenceLevel::Secondary,
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConfidenceLevel {
    /// Source quality is questionable
    Unreliable,
    /// Source has potential issues
    Questionable,
    /// A secondary source (derived from primary)
    #[default]
    Secondary,
    /// A primary source (original record)
    Primary,
    /// Direct evidence that answers the question
    Direct,
}

impl fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfidenceLevel::Unreliable => write!(f, "Unreliable"),
            ConfidenceLevel::Questionable => write!(f, "Questionable"),
            ConfidenceLevel::Secondary => write!(f, "Secondary"),
            ConfidenceLevel::Primary => write!(f, "Primary"),
            ConfidenceLevel::Direct => write!(f, "Direct"),
        }
    }
}

impl std::str::FromStr for ConfidenceLevel {
    type Err = crate::error::KinforgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unreliable" | "0" => Ok(ConfidenceLevel::Unreliable),
            "questionable" | "1" => Ok(ConfidenceLevel::Questionable),
            "secondary" | "2" => Ok(ConfidenceLevel::Secondary),
            "primary" | "3" => Ok(ConfidenceLevel::Primary),
            "direct" | "4" => Ok(ConfidenceLevel::Direct),
            _ => Err(crate::error::KinforgeError::InvalidField {
                field: "confidence".to_string(),
                value: s.to_string(),
            }),
        }
    }
}
