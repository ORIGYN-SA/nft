use ic_cdk::update;

use crate::blocks::get_all_blocks;
use crate::index::{IndexType, SortBy};
use crate::types::get_blocks;

#[update]
pub async fn get_blocks(args: get_blocks::Args) -> get_blocks::Response {
    let start = args.start;
    let length = args.length;
    let filter = args.filter;
    let sort_by = args.sort_by.unwrap_or(SortBy::Ascending);

    let mut blocks = Vec::new();

    match filter {
        Some(IndexType::Account(account)) => {
            let block_range: Vec<u64> = (start..start + length).collect(); // TODO this is wrong now, need to get all blocks for the account
            match get_all_blocks(block_range).await {
                Ok(gblocks) => {
                    blocks = gblocks;
                }
                Err(e) => {
                    ic_cdk::trap(e.to_string());
                }
            }
        }
        Some(IndexType::BlockType(block_type)) => {
            let block_range: Vec<u64> = (start..start + length).collect(); // TODO this is wrong now, need to get all blocks for the block type
            match get_all_blocks(block_range).await {
                Ok(gblocks) => {
                    blocks = gblocks;
                }
                Err(e) => {
                    ic_cdk::trap(e.to_string());
                }
            }
        }
        _ => {
            let block_range: Vec<u64> = (start..start + length).collect(); // TODO this is wrong now, need to get all blocks for the block type
            match get_all_blocks(block_range).await {
                Ok(gblocks) => {
                    blocks = gblocks;
                }
                Err(e) => {
                    ic_cdk::trap(e.to_string());
                }
            }
        }
    }

    get_blocks::Response { blocks }
}
