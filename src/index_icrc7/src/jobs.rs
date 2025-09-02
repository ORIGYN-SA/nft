use std::time::Duration;

use crate::{
    blocks::get_all_blocks,
    cache::remove_all_values_older_than,
    index::add_block_to_index,
    state::{mutate_state, read_state},
};
use bity_ic_canister_time::{run_interval, DAY_IN_MS, MINUTE_IN_MS};

const CACHE_CLEANUP_INTERVAL: u64 = DAY_IN_MS;
const UPDATE_INDEX_INTERVAL: u64 = MINUTE_IN_MS;
const BLOCK_BATCH_SIZE: u64 = 100;

pub fn start_job() {
    run_interval(Duration::from_millis(DAY_IN_MS), cleanup_cache_job);
    run_interval(
        Duration::from_millis(UPDATE_INDEX_INTERVAL),
        update_index_job,
    );
}

fn cleanup_cache_job() {
    ic_cdk::futures::spawn(cleanup_cache());
}

async fn cleanup_cache() {
    let timestamp = ic_cdk::api::time();

    remove_all_values_older_than(&timestamp);
}

fn update_index_job() {
    ic_cdk::futures::spawn(update_index());
}

async fn update_index() {
    let mut last_block_id: u64 = read_state(|state| state.data.last_block_id);

    // Generate block IDs array starting from last_block_id and incrementing
    let block_ids: Vec<u64> = (last_block_id..last_block_id + BLOCK_BATCH_SIZE).collect();

    let blocks = get_all_blocks(block_ids, None).await;
    match blocks {
        Ok(blocks) => {
            for block in blocks {
                match add_block_to_index(&block) {
                    Ok(_) => {
                        last_block_id += 1;
                    }
                    Err(e) => {
                        break;
                    }
                }
            }
        }
        Err(e) => {
            ic_cdk::trap(e);
        }
    }

    mutate_state(|state| {
        state.data.last_block_id = last_block_id;
    });
}
