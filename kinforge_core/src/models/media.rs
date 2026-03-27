use serde::{Deserialize, Serialize};
use std::fmt;

use super::ids::{MediaId, MediaLinkId};

/// A reference to an external media file (photo, document, audio, video).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Media {
    pub id: MediaId,
    /// Human-readable title or filename
    pub title: String,
    /// Local filesystem path (optional)
    pub path: Option<String>,
    /// Remote URL (optional)
    pub url: Option<String>,
    pub media_type: MediaType,
    pub description: Option<String>,
    /// Free-form date string (e.g. "1920", "circa 1920", "1920-05-01")
    pub date: Option<String>,
}

impl Media {
    pub fn new(title: impl Into<String>, media_type: MediaType) -> Self {
        Self {
            id: MediaId::new(),
            title: title.into(),
            path: None,
            url: None,
            media_type,
            description: None,
            date: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Photo,
    Document,
    Audio,
    Video,
    Other,
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediaType::Photo => write!(f, "Photo"),
            MediaType::Document => write!(f, "Document"),
            MediaType::Audio => write!(f, "Audio"),
            MediaType::Video => write!(f, "Video"),
            MediaType::Other => write!(f, "Other"),
        }
    }
}

impl std::str::FromStr for MediaType {
    type Err = crate::error::KinforgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "photo" | "image" | "picture" | "jpg" | "jpeg" | "png" => Ok(MediaType::Photo),
            "document" | "doc" | "pdf" | "text" => Ok(MediaType::Document),
            "audio" | "sound" | "mp3" | "wav" => Ok(MediaType::Audio),
            "video" | "film" | "movie" | "mp4" => Ok(MediaType::Video),
            "other" => Ok(MediaType::Other),
            _ => Err(crate::error::KinforgeError::InvalidField {
                field: "media_type".to_string(),
                value: s.to_string(),
            }),
        }
    }
}

/// Links a media record to a person, event, or source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaLink {
    pub id: MediaLinkId,
    pub media_id: MediaId,
    pub entity_type: MediaEntityType,
    /// UUID string of the linked entity
    pub entity_id: String,
}

impl MediaLink {
    pub fn new(media_id: MediaId, entity_type: MediaEntityType, entity_id: impl Into<String>) -> Self {
        Self {
            id: MediaLinkId::new(),
            media_id,
            entity_type,
            entity_id: entity_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaEntityType {
    Person,
    Event,
    Source,
}

impl fmt::Display for MediaEntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediaEntityType::Person => write!(f, "Person"),
            MediaEntityType::Event => write!(f, "Event"),
            MediaEntityType::Source => write!(f, "Source"),
        }
    }
}

impl std::str::FromStr for MediaEntityType {
    type Err = crate::error::KinforgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "person" => Ok(MediaEntityType::Person),
            "event" => Ok(MediaEntityType::Event),
            "source" => Ok(MediaEntityType::Source),
            _ => Err(crate::error::KinforgeError::InvalidField {
                field: "entity_type".to_string(),
                value: s.to_string(),
            }),
        }
    }
}
