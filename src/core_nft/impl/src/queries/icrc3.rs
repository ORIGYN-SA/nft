use crate::state::icrc3_get_archives as icrc3_get_archives_impl;
use crate::state::icrc3_get_blocks as icrc3_get_blocks_impl;
use crate::state::icrc3_get_properties as icrc3_get_properties_impl;
use crate::state::icrc3_get_tip_certificate as icrc3_get_tip_certificate_impl;
use crate::state::icrc3_supported_block_types as icrc3_supported_block_types_impl;

use ic_cdk::query;

pub use core_nft_api::types::icrc3::{
    icrc3_get_archives::{Args as GetArchivesArg, Response as GetArchivesResponse},
    icrc3_get_blocks::{Args as GetBlocksArg, Response as GetBlocksResponse},
    icrc3_get_properties::{Args as GetArchivePropsArg, Response as GetArchivePropsResponse},
    icrc3_get_tip_certificate::{
        Args as GetTipCertificateArg, Response as GetTipCertificateResponse,
    },
    icrc3_supported_block_types::{
        Args as GetSupportedBlockTypesArg, Response as GetSupportedBlockTypesResponse,
    },
};
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc3::blocks::BlockWithId;
pub use icrc_ledger_types::icrc3::blocks::GetBlocksResult;

#[query]
async fn icrc3_get_archives(_: GetArchivesArg) -> GetArchivesResponse {
    icrc3_get_archives_impl()
}

/// Block content one reply carries at most. Blocks that log whole certificates
/// (edits, and mints in permanent collections) would otherwise let a page of
/// 100 blocks pass the 2 MiB limit on a reply to another canister. Readers
/// continue from the last block id they received.
const MAX_REPLY_BLOCK_BYTES: usize = 1024 * 1024;

#[query]
fn icrc3_get_blocks(args: GetBlocksArg) -> GetBlocksResult {
    let mut result = icrc3_get_blocks_impl(args);
    keep_blocks_within(&mut result.blocks, MAX_REPLY_BLOCK_BYTES);
    result
}

/// Cuts `blocks` before the first block that takes their size past `budget`,
/// keeping at least one so a reader always moves forward.
fn keep_blocks_within(blocks: &mut Vec<BlockWithId>, budget: usize) {
    let mut used = 0usize;
    let keep = blocks
        .iter()
        .position(|block| {
            used += value_size(&block.block);
            used > budget
        })
        .unwrap_or(blocks.len())
        .max(1);
    blocks.truncate(keep);
}

/// About the size of `value` once encoded.
fn value_size(value: &ICRC3Value) -> usize {
    const OVERHEAD: usize = 8;
    OVERHEAD
        + match value {
            ICRC3Value::Blob(bytes) => bytes.len(),
            ICRC3Value::Text(text) => text.len(),
            ICRC3Value::Nat(_) | ICRC3Value::Int(_) => 16,
            ICRC3Value::Array(values) => values.iter().map(value_size).sum(),
            ICRC3Value::Map(entries) => entries
                .iter()
                .map(|(key, value)| key.len() + value_size(value))
                .sum(),
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Nat;

    fn block(id: u64, text_bytes: usize) -> BlockWithId {
        BlockWithId {
            id: Nat::from(id),
            block: ICRC3Value::Text("x".repeat(text_bytes)),
        }
    }

    #[test]
    fn blocks_stop_before_the_one_that_passes_the_budget() {
        let mut blocks = vec![block(0, 400), block(1, 400), block(2, 400)];
        keep_blocks_within(&mut blocks, 1000);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1].id, Nat::from(1u64));
    }

    #[test]
    fn one_block_is_kept_even_when_it_is_larger_than_the_budget() {
        let mut blocks = vec![block(0, 5000), block(1, 10)];
        keep_blocks_within(&mut blocks, 1000);
        assert_eq!(blocks.len(), 1);
        let mut none = vec![];
        keep_blocks_within(&mut none, 1000);
        assert!(none.is_empty());
    }

    #[test]
    fn small_blocks_all_fit() {
        let mut blocks: Vec<BlockWithId> = (0..100).map(|id| block(id, 200)).collect();
        keep_blocks_within(&mut blocks, MAX_REPLY_BLOCK_BYTES);
        assert_eq!(blocks.len(), 100);
    }
}

#[query]
async fn icrc3_get_properties(_: GetArchivePropsArg) -> GetArchivePropsResponse {
    icrc3_get_properties_impl()
}

#[query]
async fn icrc3_get_tip_certificate(_: GetTipCertificateArg) -> GetTipCertificateResponse {
    icrc3_get_tip_certificate_impl()
}

#[query]
async fn icrc3_supported_block_types(
    _: GetSupportedBlockTypesArg,
) -> GetSupportedBlockTypesResponse {
    icrc3_supported_block_types_impl()
}
