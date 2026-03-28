/// Cloud/file-based synchronisation for Kinforge data.
///
/// Sync uses an additive-only strategy: records are never deleted during a
/// pull — only records whose UUID does not exist locally are inserted.
/// This is intentionally conservative and safe for genealogy data.
///
/// # Wire format
///
/// A sync directory contains two files:
/// - `kinforge_export.json`  — full JSON snapshot of the database
/// - `kinforge_manifest.json` — metadata (device, timestamp, record counts)
use chrono::{DateTime, Utc};
use kinforge_core::{
    models::{Citation, Event, Person, Place, Relationship, Source},
    KinforgeError, KinforgeResult,
};
use kinforge_import_export::export_json;
use kinforge_storage::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Filename for the JSON data snapshot inside a sync directory.
pub const EXPORT_FILE: &str = "kinforge_export.json";

/// Filename for the sync manifest inside a sync directory.
pub const MANIFEST_FILE: &str = "kinforge_manifest.json";

// ── Internal export format mirror ─────────────────────────────────────────────

/// Mirror of `KinforgeExport` in `kinforge_import_export` (which is private).
/// Used to deserialise the remote snapshot for additive filtering.
#[derive(Deserialize)]
struct RemoteSnapshot {
    #[allow(dead_code)]
    version: String,
    people: Vec<Person>,
    events: Vec<Event>,
    places: Vec<Place>,
    relationships: Vec<Relationship>,
    sources: Vec<Source>,
    citations: Vec<Citation>,
}

// ── Manifest ─────────────────────────────────────────────────────────────────

/// Record counts stored in the manifest after a push.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestCounts {
    pub people: usize,
    pub events: usize,
    pub sources: usize,
    pub relationships: usize,
    pub places: usize,
}

/// Metadata written alongside a sync export.
///
/// The manifest lets a recipient quickly compare local vs remote counts
/// without parsing the full JSON export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    /// Opaque identifier for the device that last pushed to this directory.
    pub device_id: String,
    /// UTC timestamp of the most recent push.
    pub pushed_at: DateTime<Utc>,
    /// Version tag for the export format.
    pub export_version: String,
    /// Record counts at the time of the push.
    pub counts: ManifestCounts,
}

// ── Result types ──────────────────────────────────────────────────────────────

/// Summary returned after a push or pull operation.
#[derive(Debug, Default)]
pub struct SyncResult {
    pub records_pushed: usize,
    pub records_pulled: usize,
    /// Records skipped because they already existed locally (additive merge).
    pub duplicates_skipped: usize,
    pub conflicts: usize,
}

/// Local-vs-remote comparison returned by `sync_status`.
#[derive(Debug)]
pub struct SyncStatus {
    pub local_people: usize,
    pub local_events: usize,
    pub local_sources: usize,
    pub local_relationships: usize,
    /// People count in the remote manifest (0 if no manifest).
    pub remote_people: usize,
    /// Events count in the remote manifest (0 if no manifest).
    pub remote_events: usize,
    /// Sources count in the remote manifest (0 if no manifest).
    pub remote_sources: usize,
    /// Relationships count in the remote manifest (0 if no manifest).
    pub remote_relationships: usize,
    /// UTC timestamp of the last remote push, if a manifest exists.
    pub remote_pushed_at: Option<DateTime<Utc>>,
    /// Device ID of the last remote push, if a manifest exists.
    pub remote_device_id: Option<String>,
}

// ── Trait (extension point) ───────────────────────────────────────────────────

/// Trait implemented by sync backends (file-system, network, …).
pub trait SyncBackend: Send + Sync {
    /// Human-readable name of this backend.
    fn name(&self) -> &str;

    /// Push local database to the backend.
    fn push(&self, db: &Database) -> KinforgeResult<SyncResult>;

    /// Pull remote data from the backend into the local database (additive only).
    fn pull(&self, db: &Database) -> KinforgeResult<SyncResult>;

    /// Compare local state against the remote manifest.
    fn status(&self, db: &Database) -> KinforgeResult<SyncStatus>;
}

// ── File-system backend ───────────────────────────────────────────────────────

/// Syncs to/from a local or network-mounted directory.
///
/// The directory is shared between devices via any mechanism the user chooses
/// (USB drive, network share, cloud-folder mount such as Dropbox or Syncthing).
pub struct FileSyncBackend {
    /// Path to the shared sync directory.
    pub sync_dir: PathBuf,
    /// Stable identifier for this machine, written into the manifest on push.
    pub device_id: String,
}

impl FileSyncBackend {
    /// Create a backend with a freshly generated device ID.
    pub fn new(sync_dir: impl Into<PathBuf>) -> Self {
        Self {
            sync_dir: sync_dir.into(),
            device_id: Uuid::new_v4().to_string(),
        }
    }

    /// Create a backend with a caller-supplied device ID (stable across runs).
    pub fn with_device_id(sync_dir: impl Into<PathBuf>, device_id: impl Into<String>) -> Self {
        Self {
            sync_dir: sync_dir.into(),
            device_id: device_id.into(),
        }
    }

    fn manifest_path(&self) -> PathBuf {
        self.sync_dir.join(MANIFEST_FILE)
    }

    fn export_path(&self) -> PathBuf {
        self.sync_dir.join(EXPORT_FILE)
    }

    /// Read the manifest from the sync directory, or return `None` if absent.
    pub fn read_manifest(&self) -> KinforgeResult<Option<SyncManifest>> {
        let path = self.manifest_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content)
            .map(Some)
            .map_err(|e| KinforgeError::ImportExport(e.to_string()))
    }

    /// Parse the remote snapshot JSON.
    fn read_remote_snapshot(&self) -> KinforgeResult<RemoteSnapshot> {
        let path = self.export_path();
        if !path.exists() {
            return Err(KinforgeError::ImportExport(format!(
                "No export file found at {}: run `kinforge sync push` from another device first",
                path.display()
            )));
        }
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content)
            .map_err(|e| KinforgeError::ImportExport(e.to_string()))
    }
}

impl SyncBackend for FileSyncBackend {
    fn name(&self) -> &str {
        "file"
    }

    /// Export the full database to the sync directory and write a manifest.
    fn push(&self, db: &Database) -> KinforgeResult<SyncResult> {
        fs::create_dir_all(&self.sync_dir)?;

        // Write the JSON snapshot.
        let mut file = fs::File::create(self.export_path())?;
        export_json(db, &mut file)?;

        // Build manifest from live counts.
        let people = db.list_people()?.len();
        let events = db.list_all_events()?.len();
        let sources = db.list_sources()?.len();
        let relationships = db.list_all_relationships()?.len();
        let places = db.list_places()?.len();

        let manifest = SyncManifest {
            device_id: self.device_id.clone(),
            pushed_at: Utc::now(),
            export_version: "1.0".to_string(),
            counts: ManifestCounts {
                people,
                events,
                sources,
                relationships,
                places,
            },
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| KinforgeError::ImportExport(e.to_string()))?;
        fs::write(self.manifest_path(), manifest_json)?;

        let records_pushed = people + events + sources + relationships + places;
        Ok(SyncResult {
            records_pushed,
            records_pulled: 0,
            duplicates_skipped: 0,
            conflicts: 0,
        })
    }

    /// Import records from the sync directory that are not already present locally.
    ///
    /// Additive-only: each record's UUID is checked against the local database
    /// before insertion.  Existing records are never overwritten or deleted.
    fn pull(&self, db: &Database) -> KinforgeResult<SyncResult> {
        let remote = self.read_remote_snapshot()?;

        // Collect existing UUIDs from the local database.
        let existing_people: HashSet<String> = db
            .list_people()?
            .iter()
            .map(|p| p.id.to_string())
            .collect();
        let existing_events: HashSet<String> = db
            .list_all_events()?
            .iter()
            .map(|e| e.id.to_string())
            .collect();
        let existing_sources: HashSet<String> = db
            .list_sources()?
            .iter()
            .map(|s| s.id.to_string())
            .collect();
        let existing_rels: HashSet<String> = db
            .list_all_relationships()?
            .iter()
            .map(|r| r.id.to_string())
            .collect();
        let existing_places: HashSet<String> = db
            .list_places()?
            .iter()
            .map(|pl| pl.id.to_string())
            .collect();
        let existing_citations: HashSet<String> = db
            .list_all_citations()?
            .iter()
            .map(|c| c.id.to_string())
            .collect();

        let mut result = SyncResult::default();

        // Insert each entity type in dependency order.
        for place in &remote.places {
            if !existing_places.contains(&place.id.to_string()) {
                db.insert_place(place)?;
                result.records_pulled += 1;
            } else {
                result.duplicates_skipped += 1;
            }
        }
        for person in &remote.people {
            if !existing_people.contains(&person.id.to_string()) {
                db.insert_person(person)?;
                result.records_pulled += 1;
            } else {
                result.duplicates_skipped += 1;
            }
        }
        for source in &remote.sources {
            if !existing_sources.contains(&source.id.to_string()) {
                db.insert_source(source)?;
                result.records_pulled += 1;
            } else {
                result.duplicates_skipped += 1;
            }
        }
        for event in &remote.events {
            if !existing_events.contains(&event.id.to_string()) {
                db.insert_event(event)?;
                result.records_pulled += 1;
            } else {
                result.duplicates_skipped += 1;
            }
        }
        for rel in &remote.relationships {
            if !existing_rels.contains(&rel.id.to_string()) {
                db.insert_relationship(rel)?;
                result.records_pulled += 1;
            } else {
                result.duplicates_skipped += 1;
            }
        }
        for citation in &remote.citations {
            if !existing_citations.contains(&citation.id.to_string()) {
                db.insert_citation(citation)?;
                // citations not counted in records_pulled (no manifest field for them)
            } else {
                result.duplicates_skipped += 1;
            }
        }

        Ok(result)
    }

    /// Compare local database counts against the remote manifest.
    fn status(&self, db: &Database) -> KinforgeResult<SyncStatus> {
        let local_people = db.list_people()?.len();
        let local_events = db.list_all_events()?.len();
        let local_sources = db.list_sources()?.len();
        let local_relationships = db.list_all_relationships()?.len();

        let manifest = self.read_manifest()?;

        Ok(SyncStatus {
            local_people,
            local_events,
            local_sources,
            local_relationships,
            remote_people: manifest.as_ref().map(|m| m.counts.people).unwrap_or(0),
            remote_events: manifest.as_ref().map(|m| m.counts.events).unwrap_or(0),
            remote_sources: manifest.as_ref().map(|m| m.counts.sources).unwrap_or(0),
            remote_relationships: manifest
                .as_ref()
                .map(|m| m.counts.relationships)
                .unwrap_or(0),
            remote_pushed_at: manifest.as_ref().map(|m| m.pushed_at),
            remote_device_id: manifest.map(|m| m.device_id),
        })
    }
}

// ── Convenience free functions ────────────────────────────────────────────────

/// Push local database to `sync_dir`.  See [`FileSyncBackend::push`].
pub fn sync_push(db: &Database, sync_dir: &Path, device_id: &str) -> KinforgeResult<SyncResult> {
    FileSyncBackend::with_device_id(sync_dir, device_id).push(db)
}

/// Pull from `sync_dir` into the local database (additive only).
/// See [`FileSyncBackend::pull`].
pub fn sync_pull(db: &Database, sync_dir: &Path) -> KinforgeResult<SyncResult> {
    FileSyncBackend::new(sync_dir).pull(db)
}

/// Compare local database against the remote manifest in `sync_dir`.
pub fn sync_status(db: &Database, sync_dir: &Path) -> KinforgeResult<SyncStatus> {
    FileSyncBackend::new(sync_dir).status(db)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kinforge_app::Application;
    use kinforge_core::models::Sex;
    use std::env;

    fn temp_sync_dir() -> PathBuf {
        let dir = env::temp_dir()
            .join("kinforge_sync_tests")
            .join(Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn app() -> Application {
        Application::open_in_memory().unwrap()
    }

    // ── Test 1: push creates both files ──────────────────────────────────────

    #[test]
    fn push_creates_export_and_manifest() {
        let a = app();
        a.add_person(Some("Alice"), Some("Smith"), Sex::Female, None)
            .unwrap();

        let sync_dir = temp_sync_dir();
        let backend = FileSyncBackend::with_device_id(&sync_dir, "test-device-1");
        let result = backend.push(a.database()).unwrap();

        assert!(sync_dir.join(EXPORT_FILE).exists(), "export file missing");
        assert!(sync_dir.join(MANIFEST_FILE).exists(), "manifest file missing");
        assert!(result.records_pushed >= 1);
    }

    // ── Test 2: manifest round-trip ───────────────────────────────────────────

    #[test]
    fn manifest_round_trip() {
        let a = app();
        a.add_person(Some("Bob"), Some("Jones"), Sex::Male, None)
            .unwrap();

        let sync_dir = temp_sync_dir();
        let backend = FileSyncBackend::with_device_id(&sync_dir, "device-abc");
        backend.push(a.database()).unwrap();

        let manifest = backend.read_manifest().unwrap().expect("manifest should exist");
        assert_eq!(manifest.device_id, "device-abc");
        assert_eq!(manifest.counts.people, 1);
    }

    // ── Test 3: pull imports records into a fresh database ───────────────────

    #[test]
    fn pull_imports_records() {
        let src = app();
        src.add_person(Some("Carol"), Some("White"), Sex::Female, None)
            .unwrap();
        src.add_person(Some("Dan"), Some("Black"), Sex::Male, None)
            .unwrap();

        let sync_dir = temp_sync_dir();
        let backend = FileSyncBackend::with_device_id(&sync_dir, "src-device");
        backend.push(src.database()).unwrap();

        let dst = app();
        let result = backend.pull(dst.database()).unwrap();

        assert!(result.records_pulled >= 2, "expected at least 2 people pulled");
        let people = dst.list_people().unwrap();
        assert_eq!(people.len(), 2);
    }

    // ── Test 4: pull is additive — existing records not duplicated ────────────

    #[test]
    fn pull_is_additive_no_duplicates() {
        let src = app();
        src.add_person(Some("Eve"), Some("Green"), Sex::Female, None)
            .unwrap();

        let sync_dir = temp_sync_dir();
        let backend = FileSyncBackend::with_device_id(&sync_dir, "src-device");
        backend.push(src.database()).unwrap();

        // Pull once — imports 1 person.
        let dst = app();
        let r1 = backend.pull(dst.database()).unwrap();
        assert_eq!(r1.records_pulled, 1);
        assert_eq!(r1.duplicates_skipped, 0);

        // Pull again — person already exists, should be skipped.
        let r2 = backend.pull(dst.database()).unwrap();
        assert_eq!(r2.records_pulled, 0);
        assert_eq!(r2.duplicates_skipped, 1);

        let people = dst.list_people().unwrap();
        assert_eq!(people.len(), 1, "duplicate record created on second pull");
    }

    // ── Test 5: status reflects remote manifest counts ────────────────────────

    #[test]
    fn status_shows_remote_counts() {
        let src = app();
        src.add_person(Some("Frank"), Some("Blue"), Sex::Male, None)
            .unwrap();
        src.add_person(Some("Gina"), Some("Red"), Sex::Female, None)
            .unwrap();

        let sync_dir = temp_sync_dir();
        let backend = FileSyncBackend::with_device_id(&sync_dir, "status-device");
        backend.push(src.database()).unwrap();

        // A different local DB with only 1 person.
        let local = app();
        local
            .add_person(Some("Hank"), Some("Grey"), Sex::Male, None)
            .unwrap();

        let status = backend.status(local.database()).unwrap();
        assert_eq!(status.local_people, 1);
        assert_eq!(status.remote_people, 2);
        assert_eq!(status.remote_device_id.as_deref(), Some("status-device"));
        assert!(status.remote_pushed_at.is_some());
    }
}
