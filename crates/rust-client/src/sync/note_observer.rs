//! Side-effect-only observer trait for per-note arrivals during sync.

use alloc::boxed::Box;

use async_trait::async_trait;

use crate::ClientError;
use crate::rpc::domain::note::SyncedNote;
use crate::sync::StateSyncUpdate;

/// Per-note + post-sync side-channel into [`crate::sync::StateSync`]. Attach via
/// `StateSync::with_note_observer(...)`. Multiple observers run independently; errors are logged
/// and do not abort the sync, except a stale chain write from [`NoteObserver::apply`].
#[async_trait(?Send)]
pub trait NoteObserver {
    /// Identifier surfaced on `tracing::warn!` events for this observer.
    fn name(&self) -> &'static str;

    /// Per-note hook. Runs before the screener verdict, so before the note's id is recomputed and
    /// its inclusion proof verified. `note` carries the note's identity, metadata and inclusion
    /// proof from the sync record, its resolved attachments (empty for a note without any) and, for
    /// a fetched public note, its body.
    ///
    /// Returns `true` to mark the enclosing block as relevant even if the screener discards it, so
    /// sync persists its header.
    async fn observe(&self, note: &SyncedNote) -> Result<bool, ClientError>;

    /// Post-sync hook, invoked once after the sync window closes. Default impl is a no-op for
    /// observers that only need `observe()`.
    ///
    /// Return the
    /// [`StoreError::DatabaseTransientError`](crate::store::StoreError::DatabaseTransientError) of
    /// a store write that the store refuses as stale. The sync then fails before it writes, and
    /// [`Client::sync_chain`](crate::Client::sync_chain) and
    /// [`Client::sync_state`](crate::Client::sync_state) fetch the chain updates again and run this
    /// hook on them. The sync logs any other error and continues.
    async fn apply(&self, _sync_update: &StateSyncUpdate) -> Result<(), ClientError> {
        Ok(())
    }
}
