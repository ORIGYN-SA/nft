use std::time::Duration;

use bity_ic_canister_time::{run_interval, DAY_IN_MS};
use bity_ic_storage_canister_api::cancel_upload;
use bity_ic_storage_canister_c2c::cancel_upload;
use tracing::{debug, info};

use crate::state::{mutate_state, read_state};

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
    let stale_threshold = DAY_IN_MS * 1_000_000;

    let all_files = read_state(|state| state.internal_filestorage.get_all_files().clone());

    for (file_path, file) in all_files {
        if file.init_timestamp + stale_threshold < now {
            let result = cancel_upload(
                file.canister,
                cancel_upload::Args {
                    file_path: file_path.clone(),
                },
            )
            .await;

            match result {
                Ok(_) => {
                    debug!("Successfully canceled upload for file {}", file_path);
                }
                Err(err) => {
                    info!("Failed to cancel upload for file {}: {:?}", file_path, err);
                }
            }
        }
    }

    let stale_premint_hashes = read_state(|state| {
        state
            .data
            .private_content_system
            .temp_file_cache
            .iter()
            .filter_map(|(hash, entry)| {
                // Directly check the entry since there is no nested map here
                if let Some(pending) = &entry.pending_upload {
                    if pending.timestamp_ns + stale_threshold < now {
                        return Some(hash.clone()); // .clone() is safer here depending on if your hash is a String or [u8; 32]
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
                    .temp_file_cache
                    .remove(&hash);
            }
        });
    }
}
