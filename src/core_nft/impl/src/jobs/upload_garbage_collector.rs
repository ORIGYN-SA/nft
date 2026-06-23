use std::time::Duration;

use bity_ic_canister_time::{run_interval, DAY_IN_MS};
use bity_ic_storage_canister_c2c::remove_file;
use core_nft_common::types::management::remove_file as remove_file_types;
use tracing::{debug, info};

use crate::state::{mutate_state, read_state};

// TTL after which stale (never-finalized) uploads are cancelled: 1 day.
const STALE_UPLOAD_THRESHOLD_NS: u64 = DAY_IN_MS * 1_000_000;

// TTL after which files belonging to burned NFTs are deleted from storage: 1 day.
// Adjust this constant to tune how long burned files are retained as a safety net.
const BURNED_CONTENT_DELETE_THRESHOLD_NS: u64 = DAY_IN_MS * 1_000_000;

pub fn start_job() {
    run_interval(
        Duration::from_millis(DAY_IN_MS),
        upload_garbage_collector_job,
    );
}

fn upload_garbage_collector_job() {
    ic_cdk::futures::spawn(upload_garbage_collector());
}

async fn upload_garbage_collector() {
    let now = ic_cdk::api::time();

    // ── 1. Cancel stale (never-finalized) uploads ────────────────────────────
    let all_files = read_state(|state| state.data.public_content_system.get_all_files().clone());

    for (file_path, file) in all_files {
        if file.created_at_ns + STALE_UPLOAD_THRESHOLD_NS < now {
            let result = bity_ic_storage_canister_c2c::cancel_upload(
                file.storage_canister_id,
                bity_ic_storage_canister_api::updates::cancel_upload::Args {
                    file_path: file_path.clone(),
                },
            )
            .await;

            match result {
                Ok(_) => {
                    debug!("Successfully canceled stale upload for file {}", file_path);
                }
                Err(err) => {
                    info!(
                        "Failed to cancel stale upload for file {}: {:?}",
                        file_path, err
                    );
                }
            }
        }
    }

    // ── 2. GC stale private premint cache entries ────────────────────────────
    let stale_premint_hashes = read_state(|state| {
        state
            .data
            .private_content_system
            .premint_cache
            .iter()
            .filter_map(|(hash, entry)| {
                if let Some(pending) = &entry.pending_upload {
                    if pending.timestamp_ns + STALE_UPLOAD_THRESHOLD_NS < now {
                        return Some(*hash);
                    }
                }
                None
            })
            .collect::<Vec<_>>()
    });

    if !stale_premint_hashes.is_empty() {
        mutate_state(|state| {
            for hash in stale_premint_hashes {
                state
                    .data
                    .private_content_system
                    .premint_cache
                    .remove(&hash);
            }
        });
    }

    // ── 3. GC stale public premint cache entries ─────────────────────────────
    let stale_public_premint_hashes = read_state(|state| {
        state
            .data
            .public_content_system
            .premint_cache
            .iter()
            .filter_map(|(hash, entry)| {
                if let Some(pending) = &entry.pending_upload {
                    if pending.timestamp_ns + STALE_UPLOAD_THRESHOLD_NS < now {
                        return Some(hash.clone());
                    }
                }
                None
            })
            .collect::<Vec<_>>()
    });

    if !stale_public_premint_hashes.is_empty() {
        mutate_state(|state| {
            for hash in stale_public_premint_hashes {
                state.data.public_content_system.premint_cache.remove(&hash);
            }
        });
    }

    // ── 4. Delete files belonging to burned NFTs (public content) ────────────
    let expired_public = read_state(|state| {
        state
            .data
            .public_content_system
            .collect_expired_burned(now, BURNED_CONTENT_DELETE_THRESHOLD_NS)
    });

    let mut removed_public_token_ids = Vec::new();
    for (token_id, files) in expired_public {
        let mut all_ok = true;
        for (_entry_name, storage_canister_id, storage_path) in files {
            let result = remove_file(
                storage_canister_id,
                remove_file_types::Args {
                    file_path: storage_path.clone(),
                },
            )
            .await;
            match result {
                Ok(_) => {
                    debug!(
                        "Deleted burned public content file {} for token {}",
                        storage_path, token_id
                    );
                }
                Err(err) => {
                    info!(
                        "Failed to delete burned public content file {} for token {}: {:?}",
                        storage_path, token_id, err
                    );
                    all_ok = false;
                }
            }
        }
        if all_ok {
            removed_public_token_ids.push(token_id);
        }
    }

    if !removed_public_token_ids.is_empty() {
        mutate_state(|state| {
            for token_id in &removed_public_token_ids {
                state
                    .data
                    .public_content_system
                    .remove_burned_record(token_id);
            }
        });
    }

    // ── 5. Delete files belonging to burned NFTs (private content) ───────────
    let expired_private = read_state(|state| {
        state
            .data
            .private_content_system
            .collect_expired_burned(now, BURNED_CONTENT_DELETE_THRESHOLD_NS)
    });

    let mut removed_private_token_ids = Vec::new();
    for (token_id, files) in expired_private {
        let mut all_ok = true;
        for (_entry_name, storage_canister_id, storage_path) in files {
            let result = remove_file(
                storage_canister_id,
                remove_file_types::Args {
                    file_path: storage_path.clone(),
                },
            )
            .await;
            match result {
                Ok(_) => {
                    debug!(
                        "Deleted burned private content file {} for token {}",
                        storage_path, token_id
                    );
                }
                Err(err) => {
                    info!(
                        "Failed to delete burned private content file {} for token {}: {:?}",
                        storage_path, token_id, err
                    );
                    all_ok = false;
                }
            }
        }
        if all_ok {
            removed_private_token_ids.push(token_id);
        }
    }

    if !removed_private_token_ids.is_empty() {
        mutate_state(|state| {
            for token_id in &removed_private_token_ids {
                state
                    .data
                    .private_content_system
                    .remove_burned_record(token_id);
            }
        });
    }
}

// /// Trigger this function inside an automated timer/heartbeat loop to purge expired content.
// pub async fn process_burned_garbage_collection() {
//     let now = ic_cdk::api::time();
//     let one_day_ns = 24 * 60 * 60 * 1000 * 1000 * 1000;

//     // 1. Gather all files ready for physical deletion
//     let targets = read_state(|state| {
//         state
//             .data
//             .public_content_system
//             .collect_expired_burned(now, one_day_ns)
//     });

//     for (token_id, files_to_delete) in targets {
//         let mut all_files_deleted_successfully = true;

//         for (canister_id, path) in files_to_delete {
//             // 2. Perform the cross-canister call to delete the file bytes
//             // Assumes your storage canister exposes a `cancel_upload` or `delete_file` endpoint accepting a path String
//             let call_result: Result<((),), _> = ic_cdk::call(
//                 canister_id,
//                 "cancel_upload", // Or whatever your target deletion/cleanup string method is named
//                 (path.clone(),),
//             )
//             .await;

//             match call_result {
//                 Ok(_) => {
//                     trace(&format!("Successfully deleted remote file: {}", path));
//                 }
//                 Err(e) => {
//                     all_files_deleted_successfully = false;
//                     trace(&format!("Failed to delete remote asset {}: {:?}", path, e));
//                 }
//             }
//         }

//         // 3. Clear the records locally only if the remote canister confirmed removal
//         if all_files_deleted_successfully {
//             mutate_state(|state| {
//                 state
//                     .data
//                     .public_content_system
//                     .remove_burned_record(&token_id);
//             });
//         }
//     }
// }
