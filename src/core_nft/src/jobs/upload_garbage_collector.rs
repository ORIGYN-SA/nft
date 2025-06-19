use std::time::Duration;

use bity_ic_canister_time::{run_interval, DAY_IN_MS};
use bity_ic_storage_canister_api::cancel_upload;
use bity_ic_storage_canister_c2c::cancel_upload;
use tracing::{debug, info};

use crate::state::read_state;

pub fn start_job() {
    run_interval(
        Duration::from_millis(DAY_IN_MS),
        upload_garbage_collector_job,
    );
}

fn upload_garbage_collector_job() {
    ic_cdk::spawn(upload_garbage_collector())
}

async fn upload_garbage_collector() {
    let all_files = read_state(|state| state.internal_filestorage.get_all_files().clone());

    for (file_path, file) in all_files {
        if file.init_timestamp + DAY_IN_MS > ic_cdk::api::time() {
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
}
