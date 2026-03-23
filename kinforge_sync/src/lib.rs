/// Synchronization support for Kinforge data.
///
/// This module provides the foundational traits for future sync implementations.
/// Sync is intentionally not implemented yet — local-first is the primary design.
use kinforge_core::KinforgeResult;

/// Trait for implementing a sync backend.
pub trait SyncBackend: Send + Sync {
    /// Identifier for this backend.
    fn name(&self) -> &str;

    /// Push local changes to the remote.
    fn push(&self) -> KinforgeResult<SyncResult>;

    /// Pull remote changes into local.
    fn pull(&self) -> KinforgeResult<SyncResult>;
}

#[derive(Debug, Default)]
pub struct SyncResult {
    pub records_pushed: usize,
    pub records_pulled: usize,
    pub conflicts: usize,
}
