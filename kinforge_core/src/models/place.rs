use serde::{Deserialize, Serialize};

use super::ids::PlaceId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub id: PlaceId,
    pub name: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub parent_id: Option<PlaceId>,
}

impl Place {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: PlaceId::new(),
            name: name.into(),
            latitude: None,
            longitude: None,
            parent_id: None,
        }
    }
}
