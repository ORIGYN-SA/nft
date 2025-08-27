use crate::memory::{get_cache_memory, VM};
use crate::wrapped_values::WrappedAccount;

use crate::blocks::{get_block_instance, BlockType};
use crate::state::read_state;
use bity_ic_icrc3_archive_c2c_client::icrc3_get_blocks as archive_get_blocks;
use bity_ic_icrc3_c2c_client::icrc3_get_blocks;
use candid::{CandidType, Nat};
use ic_stable_structures::{storable::Bound, StableBTreeMap, Storable};
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc3::blocks::{BlockWithId, GetBlocksRequest};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::str::FromStr;

#[derive(
    CandidType, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq, Clone, Encode, Decode,
)]
pub enum SortBy {
    #[n(0)]
    Ascending,
    #[n(1)]
    Descending,
}

#[derive(
    CandidType, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq, Clone, Encode, Decode,
)]
pub enum IndexType {
    #[n(0)]
    Account(#[n(0)] WrappedAccount),
    #[n(1)]
    BlockType(#[n(0)] String),
    // ....
}

pub struct IndexValue(pub Vec<u64>);

thread_local! {
pub static __INDEX: std::cell::RefCell<Index> = std::cell::RefCell::new(init_index());
}

pub type Index = StableBTreeMap<IndexType, IndexValue, VM>;

pub fn init_index() -> Index {
    let memory = get_cache_memory();
    StableBTreeMap::init(memory)
}

impl Storable for IndexType {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buffer = Vec::new();
        minicbor::encode(self, &mut buffer).expect("failed to encode CustomValue");
        Cow::Owned(buffer)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        minicbor::decode(&bytes).expect("failed to decode CustomValue")
    }
    const BOUND: Bound = Bound::Unbounded;
}

impl Storable for IndexValue {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buffer = Vec::new();
        minicbor::encode(&self.0, &mut buffer).expect("failed to encode CustomValue");
        Cow::Owned(buffer)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let index_value = minicbor::decode(&bytes).expect("failed to decode CustomValue");
        IndexValue(index_value)
    }

    const BOUND: Bound = Bound::Unbounded;
}

pub fn add_block_to_index(block: &BlockWithId) -> Result<(), String> {
    let data = &block.block;

    let block_type = if let ICRC3Value::Map(map) = data {
        if let Some(ICRC3Value::Text(btype_str)) = map.get("btype") {
            BlockType::from_str(btype_str)?
        } else {
            return Err("Missing or invalid block type field".to_string());
        }
    } else {
        return Err("Invalid block data".to_string());
    };

    let block_instance = get_block_instance(&block_type);
    let accounts = block_instance.extract_accounts(data).unwrap_or_default();
    let _timestamp = block_instance.extract_timestamp(data).unwrap_or_default();
    let block_id = block_instance.extract_block_id(data).unwrap_or_default();

    if Nat::from(block_id) != block.id {
        return Err(format!("Block ID mismatch: {} != {}", block_id, block.id));
    }

    __INDEX.with(|index| {
        let mut index_mut = index.borrow_mut();

        for account in &accounts {
            let account_key = IndexType::Account(account.clone());
            let account_values = index_mut
                .get(&account_key)
                .map(|v| v.0.clone())
                .unwrap_or_default();
            let mut d: VecDeque<_> = account_values.into();
            d.push_front(block_id);
            let account_values: Vec<_> = d.into();
            index_mut.insert(account_key, IndexValue(account_values));
        }

        let block_type_key = IndexType::BlockType(block_type.to_string());
        let block_type_values = index_mut
            .get(&block_type_key)
            .map(|v| v.0.clone())
            .unwrap_or_default();
        let mut d: VecDeque<_> = block_type_values.into();
        d.push_front(block_id);
        let block_type_values: Vec<_> = d.into();
        index_mut.insert(block_type_key, IndexValue(block_type_values));
    });

    Ok(())
}
