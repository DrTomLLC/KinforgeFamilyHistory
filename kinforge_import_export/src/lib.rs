pub mod gedcom;
pub mod json;

pub use gedcom::{export_gedcom, import_gedcom};
pub use json::{export_json, import_json};

/// Statistics returned after any import operation.
#[derive(Debug, Default)]
pub struct ImportStats {
    pub people: usize,
    pub events: usize,
    pub sources: usize,
    pub relationships: usize,
    pub places: usize,
    pub duplicates_skipped: usize,
}
