use self::setup::{TestEnv, TestEnvBuilder};
use bity_ic_types::BuildVersion;
use candid::{Nat, Principal};
use core_nft::init::{InitApprovalsArg, InitArgs};
use core_nft::types::permissions::{Permission, PermissionManager};
use index_icrc7::lifecycle::InitArgs as IndexInitArgs;
use std::collections::HashMap;

pub mod setup;
pub mod setup_core;
pub mod setup_index;

pub fn default_test_setup() -> TestEnv {
    let mut test_env = TestEnvBuilder::new();

    let mut user_permissions = HashMap::new();
    user_permissions.insert(
        test_env.controller.clone(),
        vec![
            Permission::Minting,
            Permission::ManageAuthorities,
            Permission::UpdateMetadata,
            Permission::UpdateCollectionMetadata,
            Permission::ReadUploads,
            Permission::UpdateUploads,
        ],
    );

    let init_args_index = IndexInitArgs {
        test_mode: true,
        version: BuildVersion::min(),
        commit_hash: "commit_hash".to_string(),
        authorized_principals: vec![test_env.controller.clone()],
        ledger_canister_id: Principal::anonymous(),
    };

    let init_args_collection = InitArgs {
        test_mode: true,
        version: BuildVersion::min(),
        commit_hash: "commit_hash".to_string(),
        permissions: PermissionManager::new(user_permissions),
        description: None,
        symbol: "MC".to_string(),
        name: "MyCollection".to_string(),
        logo: None,
        supply_cap: Some(Nat::from(10u64)),
        max_query_batch_size: None,
        max_update_batch_size: None,
        max_take_value: None,
        default_take_value: None,
        max_memo_size: None,
        atomic_batch_transfers: None,
        tx_window: None,
        permitted_drift: None,
        max_canister_storage_threshold: None,
        collection_metadata: HashMap::new(),
        approval_init: InitApprovalsArg {
            max_approvals_per_token_or_collection: Some(Nat::from(10u64)),
            max_revoke_approvals: Some(Nat::from(10u64)),
        },
    };

    test_env.build(init_args_index, init_args_collection)
}

pub fn test_setup_atomic_batch_transfers() -> TestEnv {
    let mut test_env = TestEnvBuilder::new();

    let mut user_permissions = HashMap::new();
    user_permissions.insert(
        test_env.controller.clone(),
        vec![
            Permission::Minting,
            Permission::ManageAuthorities,
            Permission::UpdateMetadata,
            Permission::UpdateCollectionMetadata,
            Permission::ReadUploads,
            Permission::UpdateUploads,
        ],
    );

    let init_args_index = IndexInitArgs {
        test_mode: true,
        version: BuildVersion::min(),
        commit_hash: "commit_hash".to_string(),
        authorized_principals: vec![test_env.controller.clone()],
        ledger_canister_id: Principal::anonymous(),
    };

    let init_args_collection = InitArgs {
        test_mode: true,
        version: BuildVersion::min(),
        commit_hash: "commit_hash".to_string(),
        permissions: PermissionManager::new(user_permissions),
        description: None,
        symbol: "MC".to_string(),
        name: "MyCollection".to_string(),
        logo: None,
        supply_cap: None,
        max_query_batch_size: None,
        max_update_batch_size: None,
        max_take_value: None,
        default_take_value: None,
        max_memo_size: None,
        atomic_batch_transfers: Some(true),
        tx_window: None,
        permitted_drift: None,
        max_canister_storage_threshold: None,
        collection_metadata: HashMap::new(),
        approval_init: InitApprovalsArg {
            max_approvals_per_token_or_collection: Some(Nat::from(10u64)),
            max_revoke_approvals: Some(Nat::from(10u64)),
        },
    };

    test_env.build(init_args_index, init_args_collection)
}

pub fn test_setup_no_limit() -> TestEnv {
    let mut test_env = TestEnvBuilder::new();

    let mut user_permissions = HashMap::new();
    user_permissions.insert(
        test_env.controller.clone(),
        vec![
            Permission::Minting,
            Permission::ManageAuthorities,
            Permission::UpdateMetadata,
            Permission::UpdateCollectionMetadata,
            Permission::ReadUploads,
            Permission::UpdateUploads,
        ],
    );

    let init_args_index = IndexInitArgs {
        test_mode: true,
        version: BuildVersion::min(),
        commit_hash: "commit_hash".to_string(),
        authorized_principals: vec![test_env.controller.clone()],
        ledger_canister_id: Principal::anonymous(),
    };

    let init_args_collection = InitArgs {
        test_mode: true,
        version: BuildVersion::min(),
        commit_hash: "commit_hash".to_string(),
        permissions: PermissionManager::new(user_permissions),
        description: None,
        symbol: "MC".to_string(),
        name: "MyCollection".to_string(),
        logo: None,
        supply_cap: None,
        max_query_batch_size: None,
        max_update_batch_size: None,
        max_take_value: None,
        default_take_value: None,
        max_memo_size: None,
        atomic_batch_transfers: None,
        tx_window: None,
        permitted_drift: None,
        max_canister_storage_threshold: None,
        collection_metadata: HashMap::new(),
        approval_init: InitApprovalsArg {
            max_approvals_per_token_or_collection: Some(Nat::from(10u64)),
            max_revoke_approvals: Some(Nat::from(10u64)),
        },
    };

    test_env.build(init_args_index, init_args_collection)
}
