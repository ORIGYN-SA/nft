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

    // Collect stale public uploads
    let stale_public_uploads = read_state(|state| {
        state
            .data
            .public_content_system
            .temp_file_cache
            .iter()
            .filter_map(|(key, entry)| {
                if let Some(pending) = &entry.pending_upload {
                    if pending.timestamp_ns + stale_threshold < now {
                        return Some((
                            entry.storage_canister_id.clone(),
                            entry.storage_path.clone(),
                            key.clone(),
                        ));
                    }
                }
                None
            })
            .collect::<Vec<_>>()
    });

    // Cancel and remove stale public uploads
    for (canister, storage_path, key) in stale_public_uploads {
        let result = cancel_upload(
            canister,
            cancel_upload::Args {
                file_path: storage_path.clone(),
            },
        )
        .await;

        match result {
            Ok(_) => {
                debug!("Successfully canceled public upload for file {}", storage_path);
            }
            Err(err) => {
                info!("Failed to cancel public upload for file {}: {:?}", storage_path, err);
            }
        }

        mutate_state(|state| {
            state
                .data
                .public_content_system
                .temp_file_cache
                .remove(&key);
        });
    }

    // Collect stale private uploads
    let stale_private_uploads = read_state(|state| {
        state
            .data
            .private_content_system
            .temp_file_cache
            .iter()
            .filter_map(|(key, entry)| {
                if let Some(pending) = &entry.pending_upload {
                    if pending.timestamp_ns + stale_threshold < now {
                        return Some((
                            entry.storage_canister_id.clone(),
                            entry.storage_path.clone(),
                            key.clone(),
                        ));
                    }
                }
                None
            })
            .collect::<Vec<_>>()
    });

    // Cancel and remove stale private uploads
    for (canister, storage_path, key) in stale_private_uploads {
        let result = cancel_upload(
            canister,
            cancel_upload::Args {
                file_path: storage_path.clone(),
            },
        )
        .await;

        match result {
            Ok(_) => {
                debug!("Successfully canceled private upload for file {}", storage_path);
            }
            Err(err) => {
                info!("Failed to cancel private upload for file {}: {:?}", storage_path, err);
            }
        }

        mutate_state(|state| {
            state
                .data
                .private_content_system
                .temp_file_cache
                .remove(&key);
        });
    }
}
