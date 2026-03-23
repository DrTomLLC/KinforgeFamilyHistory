use thiserror::Error;

#[derive(Debug, Error)]
pub enum KinforgeError {
    #[error("Entity not found: {entity_type} with id {id}")]
    NotFound { entity_type: String, id: String },

    #[error("Invalid field value: {field} = '{value}'")]
    InvalidField { field: String, value: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Import/export error: {0}")]
    ImportExport(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type KinforgeResult<T> = Result<T, KinforgeError>;
