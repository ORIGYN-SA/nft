use crate::memory::{get_collection_approvals_memory, get_token_approvals_memory, VM};
use core_nft_common::wrapped_types::{WrappedAccount, WrappedApprovalValue, WrappedNat};
use core_nft_common::Approval;
use ic_stable_structures::StableBTreeMap;
use std::collections::HashMap;

pub const DEFAULT_MAX_APPROVALS_PER_TOKEN_OR_COLLECTION: usize = 10;

thread_local! {
    pub static __TOKEN_APPROVALS: std::cell::RefCell<TokenApprovals> = std::cell::RefCell::new(init_token_approvals());
    pub static __COLLECTION_APPROVALS: std::cell::RefCell<CollectionApprovals> = std::cell::RefCell::new(init_collection_approvals());
}

pub type TokenApprovalValue = HashMap<WrappedAccount, Approval>;
pub type TokenApprovals = StableBTreeMap<WrappedNat, WrappedApprovalValue, VM>;

pub fn init_token_approvals() -> TokenApprovals {
    let memory = get_token_approvals_memory();
    StableBTreeMap::init(memory)
}

// Map to store collection approvals: spender -> approval
pub type CollectionApprovals = StableBTreeMap<WrappedAccount, WrappedApprovalValue, VM>;

pub fn init_collection_approvals() -> CollectionApprovals {
    let memory = get_collection_approvals_memory();
    StableBTreeMap::init(memory)
}
