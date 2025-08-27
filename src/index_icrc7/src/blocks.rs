use crate::cache::{insert_value, try_get_value};
use crate::state::read_state;
use crate::wrapped_values::WrappedAccount;

use bity_ic_icrc3_archive_c2c_client::icrc3_get_blocks as archive_get_blocks;
use bity_ic_icrc3_c2c_client::icrc3_get_blocks;
use candid::{CandidType, Nat};
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc3::blocks::{BlockWithId, GetBlocksRequest};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(CandidType, Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlockType {
    Transfer(TransferBlock),
    TransferFrom(TransferFromBlock),
    UpdateTokenMetadata(UpdateTokenMetadataBlock),
    Mint(MintBlock),
    Burn(BurnBlock),
    Approve(ApproveBlock),
    CollectionApprove(CollectionApproveBlock),
    Revoke(RevokeBlock),
    RevokeCollection(RevokeCollectionBlock),
}

#[derive(CandidType, Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransferBlock;

#[derive(CandidType, Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MintBlock;

#[derive(CandidType, Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BurnBlock;

#[derive(CandidType, Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApproveBlock;

#[derive(CandidType, Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransferFromBlock;

#[derive(CandidType, Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpdateTokenMetadataBlock;

#[derive(CandidType, Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CollectionApproveBlock;

#[derive(CandidType, Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RevokeBlock;

#[derive(CandidType, Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RevokeCollectionBlock;

impl FromStr for BlockType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "7xfer" => Ok(BlockType::Transfer(TransferBlock)),
            "7mint" => Ok(BlockType::Mint(MintBlock)),
            "7burn" => Ok(BlockType::Burn(BurnBlock)),
            "37approve" => Ok(BlockType::Approve(ApproveBlock)),
            "37xfer" => Ok(BlockType::TransferFrom(TransferFromBlock)),
            "37approve_coll" => Ok(BlockType::CollectionApprove(CollectionApproveBlock)),
            "37revoke" => Ok(BlockType::Revoke(RevokeBlock)),
            "37revoke_coll" => Ok(BlockType::RevokeCollection(RevokeCollectionBlock)),
            "7update_token" => Ok(BlockType::UpdateTokenMetadata(UpdateTokenMetadataBlock)),
            _ => Err(format!("Unknown block type: {}", s)),
        }
    }
}

impl ToString for BlockType {
    fn to_string(&self) -> String {
        match self {
            BlockType::Transfer(_) => "7xfer".to_string(),
            BlockType::Mint(_) => "7mint".to_string(),
            BlockType::Burn(_) => "7burn".to_string(),
            BlockType::Approve(_) => "37approve".to_string(),
            BlockType::TransferFrom(_) => "37xfer".to_string(),
            BlockType::CollectionApprove(_) => "37approve_coll".to_string(),
            BlockType::Revoke(_) => "37revoke".to_string(),
            BlockType::RevokeCollection(_) => "37revoke_coll".to_string(),
            BlockType::UpdateTokenMetadata(_) => "7update_token".to_string(),
        }
    }
}

pub fn get_block_instance(block_type: &BlockType) -> Box<dyn TransactionDataExtractor> {
    match block_type {
        BlockType::Transfer(_) => Box::new(TransferBlock),
        BlockType::Mint(_) => Box::new(MintBlock),
        BlockType::Burn(_) => Box::new(BurnBlock),
        BlockType::Approve(_) => Box::new(ApproveBlock),
        BlockType::TransferFrom(_) => Box::new(TransferFromBlock),
        BlockType::CollectionApprove(_) => Box::new(CollectionApproveBlock),
        BlockType::Revoke(_) => Box::new(RevokeBlock),
        BlockType::RevokeCollection(_) => Box::new(RevokeCollectionBlock),
        BlockType::UpdateTokenMetadata(_) => Box::new(UpdateTokenMetadataBlock),
    }
}

pub trait BlockIndexer {
    fn should_index_by_timestamp(&self) -> bool;
    fn should_index_by_account(&self) -> bool;

    fn should_index_by_block_type(&self) -> bool;
    fn account_to_index(&self) -> Option<WrappedAccount>;
}

pub trait TransactionDataExtractor {
    fn extract_accounts(&self, data: &ICRC3Value) -> Result<Vec<WrappedAccount>, String>;

    fn extract_block_type(&self, data: &ICRC3Value) -> Result<BlockType, String> {
        if let ICRC3Value::Map(map) = data {
            if let Some(ICRC3Value::Text(btype_str)) = map.get("btype") {
                Ok(BlockType::from_str(btype_str).map_err(|_| "Invalid block type".to_string())?)
            } else {
                Err("Missing or invalid block type field".to_string())
            }
        } else {
            Err("Invalid block data".to_string())
        }
    }

    fn extract_timestamp(&self, data: &ICRC3Value) -> Result<u64, String> {
        if let ICRC3Value::Map(map) = data {
            if let Some(ICRC3Value::Nat(timestamp_nat)) = map.get("timestamp") {
                Ok(u64::try_from(timestamp_nat.0.clone())
                    .map_err(|_| "Invalid timestamp format")?)
            } else {
                Err("Missing or invalid timestamp field".to_string())
            }
        } else {
            Err("Invalid block data".to_string())
        }
    }

    fn extract_block_id(&self, data: &ICRC3Value) -> Result<u64, String> {
        if let ICRC3Value::Map(map) = data {
            if let Some(ICRC3Value::Nat(block_id_nat)) = map.get("id") {
                Ok(u64::try_from(block_id_nat.0.clone()).map_err(|_| "Invalid block ID format")?)
            } else {
                Err("Missing or invalid block ID field".to_string())
            }
        } else {
            Err("Invalid block data".to_string())
        }
    }
}

impl BlockIndexer for BlockType {
    fn should_index_by_timestamp(&self) -> bool {
        true
    }

    fn should_index_by_account(&self) -> bool {
        match self {
            BlockType::Transfer(_) => true,
            BlockType::Mint(_) => true,
            BlockType::Burn(_) => true,
            BlockType::Approve(_) => true,
            BlockType::TransferFrom(_) => true,
            BlockType::CollectionApprove(_) => false,
            BlockType::Revoke(_) => true,
            BlockType::RevokeCollection(_) => false,
            BlockType::UpdateTokenMetadata(_) => true,
        }
    }

    fn should_index_by_block_type(&self) -> bool {
        true
    }

    fn account_to_index(&self) -> Option<WrappedAccount> {
        None
    }
}

impl TransactionDataExtractor for TransferBlock {
    fn extract_accounts(&self, data: &ICRC3Value) -> Result<Vec<WrappedAccount>, String> {
        match data {
            ICRC3Value::Map(map) => {
                let mut accounts = Vec::new();
                if let Some(ICRC3Value::Map(tx)) = map.get("tx") {
                    if let Some(ICRC3Value::Text(from)) = tx.get("from") {
                        if let Ok(account) = WrappedAccount::from_str(from) {
                            accounts.push(account);
                        }
                    }
                    if let Some(ICRC3Value::Text(to)) = tx.get("to") {
                        if let Ok(account) = WrappedAccount::from_str(to) {
                            accounts.push(account);
                        }
                    }
                }
                Ok(accounts)
            }
            _ => Err("Transfer transaction data must be a map".to_string()),
        }
    }
}

impl TransactionDataExtractor for MintBlock {
    fn extract_accounts(&self, data: &ICRC3Value) -> Result<Vec<WrappedAccount>, String> {
        match data {
            ICRC3Value::Map(map) => {
                let mut accounts = Vec::new();
                if let Some(ICRC3Value::Map(tx)) = map.get("tx") {
                    if let Some(ICRC3Value::Text(to)) = tx.get("to") {
                        if let Ok(account) = WrappedAccount::from_str(to) {
                            accounts.push(account);
                        }
                    }
                }
                Ok(accounts)
            }
            _ => Err("Mint transaction data must be a map".to_string()),
        }
    }
}

impl TransactionDataExtractor for BurnBlock {
    fn extract_accounts(&self, data: &ICRC3Value) -> Result<Vec<WrappedAccount>, String> {
        match data {
            ICRC3Value::Map(map) => {
                let mut accounts = Vec::new();
                if let Some(ICRC3Value::Map(tx)) = map.get("tx") {
                    if let Some(ICRC3Value::Text(from)) = tx.get("from") {
                        if let Ok(account) = WrappedAccount::from_str(from) {
                            accounts.push(account);
                        }
                    }
                    if let Some(ICRC3Value::Text(to)) = tx.get("to") {
                        if let Ok(account) = WrappedAccount::from_str(to) {
                            accounts.push(account);
                        }
                    }
                }
                Ok(accounts)
            }
            _ => Err("Burn transaction data must be a map".to_string()),
        }
    }
}

impl TransactionDataExtractor for ApproveBlock {
    fn extract_accounts(&self, data: &ICRC3Value) -> Result<Vec<WrappedAccount>, String> {
        match data {
            ICRC3Value::Map(map) => {
                let mut accounts: Vec<WrappedAccount> = Vec::new();
                if let Some(ICRC3Value::Map(tx)) = map.get("tx") {
                    if let Some(ICRC3Value::Text(from)) = tx.get("from") {
                        if let Ok(account) = WrappedAccount::from_str(from) {
                            accounts.push(account);
                        }
                    }
                    if let Some(ICRC3Value::Text(spender)) = tx.get("spender") {
                        if let Ok(account) = WrappedAccount::from_str(spender) {
                            accounts.push(account);
                        }
                    }
                }
                Ok(accounts)
            }
            _ => Err("Approve transaction data must be a map".to_string()),
        }
    }
}

impl TransactionDataExtractor for TransferFromBlock {
    fn extract_accounts(&self, data: &ICRC3Value) -> Result<Vec<WrappedAccount>, String> {
        match data {
            ICRC3Value::Map(map) => {
                let mut accounts: Vec<WrappedAccount> = Vec::new();
                if let Some(ICRC3Value::Map(tx)) = map.get("tx") {
                    if let Some(ICRC3Value::Text(from)) = tx.get("from") {
                        if let Ok(account) = WrappedAccount::from_str(from) {
                            accounts.push(account);
                        }
                    }
                    if let Some(ICRC3Value::Text(spender)) = tx.get("spender") {
                        if let Ok(account) = WrappedAccount::from_str(spender) {
                            accounts.push(account);
                        }
                    }
                    if let Some(ICRC3Value::Text(to)) = tx.get("to") {
                        if let Ok(account) = WrappedAccount::from_str(to) {
                            accounts.push(account);
                        }
                    }
                }
                Ok(accounts)
            }
            _ => Err("TransferFrom transaction data must be a map".to_string()),
        }
    }
}

impl TransactionDataExtractor for UpdateTokenMetadataBlock {
    fn extract_accounts(&self, data: &ICRC3Value) -> Result<Vec<WrappedAccount>, String> {
        match data {
            ICRC3Value::Map(map) => {
                let mut accounts: Vec<WrappedAccount> = Vec::new();
                if let Some(ICRC3Value::Map(tx)) = map.get("tx") {
                    if let Some(ICRC3Value::Text(from)) = tx.get("from") {
                        if let Ok(account) = WrappedAccount::from_str(from) {
                            accounts.push(account);
                        }
                    }
                }
                Ok(accounts)
            }
            _ => Err("UpdateTokenMetadata transaction data must be a map".to_string()),
        }
    }
}

impl TransactionDataExtractor for CollectionApproveBlock {
    fn extract_accounts(&self, data: &ICRC3Value) -> Result<Vec<WrappedAccount>, String> {
        match data {
            ICRC3Value::Map(map) => {
                let mut accounts: Vec<WrappedAccount> = Vec::new();
                if let Some(ICRC3Value::Map(tx)) = map.get("tx") {
                    if let Some(ICRC3Value::Text(from)) = tx.get("from") {
                        if let Ok(account) = WrappedAccount::from_str(from) {
                            accounts.push(account);
                        }
                    }
                }
                Ok(accounts)
            }
            _ => Err("CollectionApprove transaction data must be a map".to_string()),
        }
    }
}

impl TransactionDataExtractor for RevokeBlock {
    fn extract_accounts(&self, data: &ICRC3Value) -> Result<Vec<WrappedAccount>, String> {
        match data {
            ICRC3Value::Map(map) => {
                let mut accounts: Vec<WrappedAccount> = Vec::new();
                if let Some(ICRC3Value::Map(tx)) = map.get("tx") {
                    if let Some(ICRC3Value::Text(from)) = tx.get("from") {
                        if let Ok(account) = WrappedAccount::from_str(from) {
                            accounts.push(account);
                        }
                    }
                    if let Some(ICRC3Value::Text(spender)) = tx.get("spender") {
                        if let Ok(account) = WrappedAccount::from_str(spender) {
                            accounts.push(account);
                        }
                    }
                }
                Ok(accounts)
            }
            _ => Err("Revoke transaction data must be a map".to_string()),
        }
    }
}

impl TransactionDataExtractor for RevokeCollectionBlock {
    fn extract_accounts(&self, data: &ICRC3Value) -> Result<Vec<WrappedAccount>, String> {
        match data {
            ICRC3Value::Map(map) => {
                let mut accounts: Vec<WrappedAccount> = Vec::new();
                if let Some(ICRC3Value::Map(tx)) = map.get("tx") {
                    if let Some(ICRC3Value::Text(from)) = tx.get("from") {
                        if let Ok(account) = WrappedAccount::from_str(from) {
                            accounts.push(account);
                        }
                    }
                    if let Some(ICRC3Value::Text(spender)) = tx.get("spender") {
                        if let Ok(account) = WrappedAccount::from_str(spender) {
                            accounts.push(account);
                        }
                    }
                }
                Ok(accounts)
            }
            _ => Err("RevokeCollection transaction data must be a map".to_string()),
        }
    }
}

pub async fn get_all_blocks(block_ids: Vec<u64>) -> Result<Vec<BlockWithId>, String> {
    if block_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut ret_blocks: Vec<BlockWithId> = Vec::new();
    let mut uncached_block_ids: Vec<u64> = Vec::new();

    // First, check cache for each block ID
    for &block_id in &block_ids {
        ic_cdk::println!("block_id: {:?}", block_id);
        if let Some(cached_block) = try_get_value(block_id) {
            ic_cdk::println!("cached_block: {:?}", cached_block);
            let block_with_id = BlockWithId {
                id: Nat::from(block_id),
                block: cached_block.0,
            };
            ret_blocks.push(block_with_id);
        } else {
            ic_cdk::println!("uncached_block_ids: {:?}", block_id);
            uncached_block_ids.push(block_id);
        }
    }

    // If all blocks were found in cache, return early
    if uncached_block_ids.is_empty() {
        ret_blocks.sort_by(|a, b| a.id.cmp(&b.id));
        return Ok(ret_blocks);
    }

    // Fetch only uncached blocks from network
    let mut sorted_block_ids = uncached_block_ids.clone();
    sorted_block_ids.sort();

    let chunks = group_consecutive_block_ids(&sorted_block_ids);

    ic_cdk::println!("chunks: {:?}", chunks);

    let ledger_canister_id = read_state(|state| state.data.ledger_canister_id);

    for chunk in chunks {
        let start = chunk[0];
        let length = chunk.len() as u64;

        let blocks = icrc3_get_blocks(
            ledger_canister_id,
            vec![GetBlocksRequest {
                start: Nat::from(start),
                length: Nat::from(length),
            }],
        )
        .await
        .map_err(|e| e.to_string())?;

        ic_cdk::println!("blocks: {:?}", blocks);

        for archive in blocks.archived_blocks {
            let archive_blocks = archive_get_blocks(archive.callback.canister_id, archive.args)
                .await
                .map_err(|e| e.to_string())?;

            ic_cdk::println!("archive_blocks: {:?}", archive_blocks);

            for block in archive_blocks.blocks {
                if let Ok(block_id) = u64::try_from(&block.id.0) {
                    ic_cdk::println!("block_id: {:?}", block_id);
                    insert_value(
                        block_id,
                        crate::wrapped_values::CustomValue(block.block.clone()),
                    );
                }
                ret_blocks.push(block);
            }
        }

        for block in blocks.blocks {
            if let Ok(block_id) = u64::try_from(&block.id.0) {
                insert_value(
                    block_id,
                    crate::wrapped_values::CustomValue(block.block.clone()),
                );
            }
            ret_blocks.push(block);
        }
    }

    ret_blocks.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(ret_blocks)
}

/// Groups consecutive block IDs into chunks for efficient batch processing
///
/// # Arguments
/// * `block_ids` - A sorted vector of block IDs
///
/// # Returns
/// A vector of chunks, where each chunk contains consecutive block IDs
///
/// # Example
/// ```
/// let block_ids = vec![1, 2, 3, 5, 6, 8, 9, 10];
/// let chunks = group_consecutive_block_ids(block_ids);
/// // Result: [[1, 2, 3], [5, 6], [8, 9, 10]]
/// ```
pub fn group_consecutive_block_ids(block_ids: &[u64]) -> Vec<Vec<u64>> {
    if block_ids.is_empty() {
        return Vec::new();
    }

    let mut chunks: Vec<Vec<u64>> = Vec::new();
    let mut start_idx = 0;

    for i in 1..block_ids.len() {
        if block_ids[i] != block_ids[i - 1] + 1 {
            // Non-consecutive, create chunk from start_idx to i-1
            chunks.push(block_ids[start_idx..i].to_vec());
            start_idx = i;
        }
    }

    // Add the last chunk
    chunks.push(block_ids[start_idx..].to_vec());

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_consecutive_block_ids_empty() {
        let block_ids: Vec<u64> = vec![];
        let chunks = group_consecutive_block_ids(&block_ids);
        assert_eq!(chunks, Vec::<Vec<u64>>::new());
    }

    #[test]
    fn test_group_consecutive_block_ids_single() {
        let block_ids = vec![42];
        let chunks = group_consecutive_block_ids(&block_ids);
        assert_eq!(chunks, vec![vec![42]]);
    }

    #[test]
    fn test_group_consecutive_block_ids_all_consecutive() {
        let block_ids = vec![1, 2, 3, 4, 5];
        let chunks = group_consecutive_block_ids(&block_ids);
        assert_eq!(chunks, vec![vec![1, 2, 3, 4, 5]]);
    }

    #[test]
    fn test_group_consecutive_block_ids_no_consecutive() {
        let block_ids = vec![1, 3, 5, 7, 9];
        let chunks = group_consecutive_block_ids(&block_ids);
        assert_eq!(chunks, vec![vec![1], vec![3], vec![5], vec![7], vec![9]]);
    }

    #[test]
    fn test_group_consecutive_block_ids_mixed() {
        let block_ids = vec![1, 2, 3, 5, 6, 8, 9, 10];
        let chunks = group_consecutive_block_ids(&block_ids);
        assert_eq!(chunks, vec![vec![1, 2, 3], vec![5, 6], vec![8, 9, 10]]);
    }

    #[test]
    fn test_group_consecutive_block_ids_gaps() {
        let block_ids = vec![1, 2, 5, 6, 7, 10, 11, 15];
        let chunks = group_consecutive_block_ids(&block_ids);
        assert_eq!(
            chunks,
            vec![vec![1, 2], vec![5, 6, 7], vec![10, 11], vec![15]]
        );
    }

    #[test]
    fn test_group_consecutive_block_ids_large_numbers() {
        let block_ids = vec![1000, 1001, 1002, 2000, 2001, 3000];
        let chunks = group_consecutive_block_ids(&block_ids);
        assert_eq!(
            chunks,
            vec![vec![1000, 1001, 1002], vec![2000, 2001], vec![3000]]
        );
    }

    #[test]
    fn test_group_consecutive_block_ids_edge_cases() {
        let block_ids = vec![0, 1, 2, 3];
        let chunks = group_consecutive_block_ids(&block_ids);
        assert_eq!(chunks, vec![vec![0, 1, 2, 3]]);
    }

    #[test]
    fn test_group_consecutive_block_ids_reverse_order() {
        // This test shows that the function expects sorted input
        let block_ids = vec![5, 4, 3, 2, 1];
        let chunks = group_consecutive_block_ids(&block_ids);
        // Since input is not sorted, this will create separate chunks
        assert_eq!(chunks, vec![vec![5], vec![4], vec![3], vec![2], vec![1]]);
    }
}
