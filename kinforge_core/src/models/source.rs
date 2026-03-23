use serde::{Deserialize, Serialize};

use super::ids::SourceId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub id: SourceId,
    pub title: String,
    pub author: Option<String>,
    pub publication: Option<String>,
    pub year: Option<i32>,
    pub repository: Option<String>,
    pub notes: Option<String>,
}

impl Source {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: SourceId::new(),
            title: title.into(),
            author: None,
            publication: None,
            year: None,
            repository: None,
            notes: None,
        }
    }
}
