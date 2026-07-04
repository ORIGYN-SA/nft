use bity_ic_storage_canister_api::storage::UploadState;
use core_nft_common::types::public_content::{PendingUpload, PublicEntry};
use core_nft_common::types::CHUNK_SIZE;
use std::collections::HashMap;
use tracing::info;

use crate::state::RuntimeState;

/// Makes files uploaded by pre-content-system versions of this canister visible
/// to the current upload APIs (`get_upload_status`, `get_all_uploads`,
/// `cancel_upload`, ...).
///
/// Older versions tracked uploads only in `internal_filestorage`; the content
/// systems introduced later start empty after an upgrade, so without this step
/// every pre-upgrade file would answer `UploadNotFound` even though it is still
/// stored and served. Idempotent: paths already known to the public content
/// system are left untouched, so re-running on every upgrade is safe.
///
/// * `Finalized` entries become finalized `temp_file_cache` entries (the same
///   shape as a finalized-but-unminted upload). Their byte size was never
///   recorded by the old bookkeeping, so `file_size` is 0.
/// * `Init`/`InProgress` entries are abandoned pre-upgrade uploads. They are
///   registered with their original timestamp and an empty pending upload, so
///   the daily upload garbage collector cancels them and frees their paths.
pub fn migrate_internal_filestorage_into_public_content(state: &mut RuntimeState) {
    let entries: Vec<_> = state
        .internal_filestorage
        .map
        .iter()
        .map(|(path, data)| (path.clone(), data.clone()))
        .collect();

    let mut migrated = 0usize;

    for (path, entry) in entries {
        let already_known = state
            .data
            .public_content_system
            .get_public_file_by_path(&path)
            .is_some()
            || state
                .data
                .public_content_system
                .file_to_nfts
                .contains_key(&path);

        if already_known {
            continue;
        }

        let pending_upload = match entry.state {
            UploadState::Finalized => None,
            _ => Some(PendingUpload {
                expected_chunks: 0,
                received_chunks: HashMap::new(),
                chunk_size: CHUNK_SIZE,
                timestamp_ns: entry.init_timestamp,
            }),
        };

        let public_entry = PublicEntry {
            state: entry.state.clone(),
            hash: path.clone(),
            file_size: 0,
            storage_canister_id: entry.canister,
            storage_path: entry.path.clone(),
            pending_upload,
            format_version: 1,
            created_at_ns: entry.init_timestamp,
        };

        state
            .data
            .public_content_system
            .temp_file_cache
            .insert(path, public_entry);

        migrated += 1;
    }

    if migrated > 0 {
        info!("Migrated {migrated} legacy file entries into the public content system");
    }
}
