use serde::{Deserialize, Serialize};
use std::fmt;

use super::ids::{PersonId, TaskId};

/// A genealogy research task (to-do item).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub description: String,
    /// Optional link to a specific person this task is about
    pub person_id: Option<PersonId>,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub notes: Option<String>,
    /// ISO-8601 date-time string when the task was created
    pub created: String,
    /// ISO-8601 date-time string when the task was last updated
    pub updated: String,
}

impl Task {
    pub fn new(description: impl Into<String>) -> Self {
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        Self {
            id: TaskId::new(),
            description: description.into(),
            person_id: None,
            priority: TaskPriority::Medium,
            status: TaskStatus::Pending,
            notes: None,
            created: now.clone(),
            updated: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskPriority::Low => write!(f, "Low"),
            TaskPriority::Medium => write!(f, "Medium"),
            TaskPriority::High => write!(f, "High"),
        }
    }
}

impl std::str::FromStr for TaskPriority {
    type Err = crate::error::KinforgeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" | "l" => Ok(TaskPriority::Low),
            "medium" | "med" | "m" => Ok(TaskPriority::Medium),
            "high" | "h" => Ok(TaskPriority::High),
            _ => Err(crate::error::KinforgeError::InvalidField {
                field: "priority".to_string(),
                value: s.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::InProgress => write!(f, "InProgress"),
            TaskStatus::Done => write!(f, "Done"),
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = crate::error::KinforgeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(['-', '_'], "").as_str() {
            "pending" | "todo" | "open" => Ok(TaskStatus::Pending),
            "inprogress" | "wip" | "active" | "started" => Ok(TaskStatus::InProgress),
            "done" | "complete" | "completed" | "closed" => Ok(TaskStatus::Done),
            _ => Err(crate::error::KinforgeError::InvalidField {
                field: "status".to_string(),
                value: s.to_string(),
            }),
        }
    }
}
