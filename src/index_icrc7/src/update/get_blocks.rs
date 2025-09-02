use ic_cdk::update;

use crate::blocks::get_all_blocks;
use crate::index::{IndexType, IndexValue, SortBy, __INDEX};
use crate::state::read_state;
use crate::types::get_blocks;

#[update]
pub async fn get_blocks(args: get_blocks::Args) -> get_blocks::Response {
    let mut start = args.start;
    let length = args.length;
    let filters = args.filters;
    let sort_by = args.sort_by.unwrap_or(SortBy::Ascending);

    let blocks = if filters.is_empty() {
        if sort_by == SortBy::Descending {
            start = read_state(|state| state.data.last_block_id).saturating_sub(length);
        }

        let block_range: Vec<u64> = (start..start + length).collect();
        match get_all_blocks(block_range, Some(sort_by.clone())).await {
            Ok(gblocks) => gblocks,
            Err(e) => {
                ic_cdk::trap(e.to_string());
            }
        }
    } else {
        get_blocks_with_filters(&filters, start, length, &sort_by).await
    };

    get_blocks::Response { blocks }
}

async fn get_blocks_with_filters(
    filters: &[IndexType],
    start: u64,
    length: u64,
    sort_by: &SortBy,
) -> Vec<icrc_ledger_types::icrc3::blocks::BlockWithId> {
    let mut filter_results: Vec<Vec<u64>> = Vec::new();

    for filter in filters {
        let block_ids = __INDEX.with(|index| {
            let index_ref = index.borrow();
            if let Some(IndexValue(block_ids)) = index_ref.get(filter) {
                block_ids.clone()
            } else {
                Vec::new()
            }
        });
        filter_results.push(block_ids);
    }

    let mut combined_block_ids = if filter_results.is_empty() {
        Vec::new()
    } else {
        let mut result = filter_results[0].clone();

        for filter_result in filter_results.iter().skip(1) {
            result.retain(|id| filter_result.contains(id));
        }

        result
    };

    combined_block_ids.sort();

    let start_idx = start as usize;
    let end_idx = (start + length) as usize;

    if start_idx >= combined_block_ids.len() {
        return Vec::new();
    }

    let end_idx = end_idx.min(combined_block_ids.len());
    let paginated_block_ids = &combined_block_ids[start_idx..end_idx];

    let block_range: Vec<u64> = paginated_block_ids.to_vec();
    match get_all_blocks(block_range, Some(sort_by.clone())).await {
        Ok(blocks) => blocks,
        Err(e) => {
            ic_cdk::trap(e.to_string());
        }
    }
}
