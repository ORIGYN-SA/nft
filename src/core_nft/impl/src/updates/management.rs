use crate::guards::{
    caller_has_manage_authorities_permission, caller_has_minting_permission,
    caller_has_read_uploads_permission, caller_has_update_collection_metadata_permission,
    caller_has_update_metadata_permission, caller_has_update_uploads_permission, GuardManagement,
};
use crate::state::{icrc3_add_transaction, mutate_state, read_state, InternalFilestorageData};
use crate::types::metadata::__METADATA;
use crate::types::nft::Icrc7Token;
use crate::utils::check_memo;
use core_nft_common::CHUNK_SIZE;

#[cfg(feature = "inttest")]
pub use core_nft_api::__get_public_entry_test;
use core_nft_common::resolve_readers;
use core_nft_common::types::http::add_redirection;
use core_nft_common::types::sub_canister::StorageCanister;
use core_nft_common::types::{icrc7, management};
use core_nft_common::utils::trace;
use core_nft_common::MAX_PRIVATE_CONTENT_SIZE;

use bity_ic_icrc3::transaction::{ICRC7Transaction, ICRC7TransactionData};
use bity_ic_storage_canister_api::types::storage::UploadState;
pub use candid::{Nat, Principal};
pub use core_nft_common::types::management::{
    cancel_upload, finalize_upload, get_user_permissions, grant_permission, has_permission,
    init_upload, migration_icrc3_add_transaction, revoke_permission, store_chunk,
};
pub use core_nft_common::types::permissions::Permission;
use core_nft_common::{construct_canonical_identity, PrivateContentStatus};
pub use ic_cdk::call::RejectCode;
use ic_cdk_macros::{query, update};
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use icrc_ledger_types::icrc1::account::Account;
use std::collections::BTreeMap;
use std::collections::HashMap;

#[update(guard = "caller_has_update_collection_metadata_permission")]
pub async fn update_collection_metadata(
    req: management::update_collection_metadata::Args,
) -> management::update_collection_metadata::Response {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller)
        .map_err(|_| management::update_collection_metadata::UpdateCollectionMetadataError::ConcurrentManagementCall)?;

    if let Some(description) = req.description {
        mutate_state(|state| {
            state.data.description = Some(description);
        });
    }

    if let Some(symbol) = req.symbol {
        mutate_state(|state| {
            state.data.symbol = symbol;
        });
    }

    if let Some(name) = req.name {
        mutate_state(|state| {
            state.data.name = name;
        });
    }

    if let Some(logo) = req.logo {
        mutate_state(|state| {
            state.data.logo = Some(logo);
        });
    }

    if let Some(supply_cap) = req.supply_cap {
        mutate_state(|state| {
            state.data.supply_cap = Some(supply_cap);
        });
    }

    if let Some(max_query_batch_size) = req.max_query_batch_size {
        mutate_state(|state| {
            state.data.max_query_batch_size = Some(max_query_batch_size);
        });
    }

    if let Some(max_update_batch_size) = req.max_update_batch_size {
        mutate_state(|state| {
            state.data.max_update_batch_size = Some(max_update_batch_size);
        });
    }

    if let Some(max_take_value) = req.max_take_value {
        mutate_state(|state| {
            state.data.max_take_value = Some(max_take_value);
        });
    }

    if let Some(default_take_value) = req.default_take_value {
        mutate_state(|state| {
            state.data.default_take_value = Some(default_take_value);
        });
    }

    if let Some(max_memo_size) = req.max_memo_size {
        mutate_state(|state| {
            state.data.max_memo_size = Some(max_memo_size);
        });
    }

    if let Some(atomic_batch_transfers) = req.atomic_batch_transfers {
        mutate_state(|state| {
            state.data.atomic_batch_transfers = Some(atomic_batch_transfers);
        });
    }

    if let Some(tx_window) = req.tx_window {
        mutate_state(|state| {
            state.data.tx_window = Some(tx_window);
        });
    }

    if let Some(permitted_drift) = req.permitted_drift {
        mutate_state(|state| {
            state.data.permitted_drift = Some(permitted_drift);
        });
    }

    if let Some(max_canister_storage_threshold) = req.max_canister_storage_threshold {
        mutate_state(|state| {
            state.data.max_canister_storage_threshold = Some(max_canister_storage_threshold);
        });
    }

    if let Some(custom_collection_metadata) = req.collection_metadata {
        for key in custom_collection_metadata.keys() {
            if key.starts_with("icrc7:") {
                return Err(management::update_collection_metadata::UpdateCollectionMetadataError::InvalidMetadataKey(
                    format!("Cannot overwrite protected key '{}'", key)
                ));
            }
        }
        mutate_state(|state| {
            state.data.custom_collection_metadata = custom_collection_metadata;
        });
    }

    Ok(())
}

#[update(guard = "caller_has_minting_permission")]
pub fn mint(req: management::mint::Args) -> management::mint::Response {
    trace("Minting NFT batch");
    trace(&format!("timestamp: {:?}", ic_cdk::api::time()));
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller)
        .map_err(|_| management::mint::MintError::ConcurrentManagementCall)?;

    // Efficiently read config and state without cloning the whole ledger
    let (max_batch_size, current_token_id, supply_cap, current_token_count) = read_state(|state| {
        (
            state
                .data
                .max_update_batch_size
                .as_ref()
                .map(|n| n.0.clone())
                .unwrap_or(100u64.into()),
            state.data.last_token_id.clone(),
            state
                .data
                .supply_cap
                .as_ref()
                .cloned()
                .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_SUPPLY_CAP)),
            state.data.tokens_list.len() as u64,
        )
    });

    if req.mint_requests.len() as u64 > u64::try_from(max_batch_size).unwrap_or(100) {
        return Err(management::mint::MintError::ExceedMaxAllowedSupplyCap);
    }

    if current_token_count + req.mint_requests.len() as u64 > supply_cap {
        return Err(management::mint::MintError::ExceedMaxAllowedSupplyCap);
    }

    for mint_request in &req.mint_requests {
        match check_memo(&mint_request.memo) {
            Ok(_) => {}
            Err(_) => {
                return Err(management::mint::MintError::InvalidMemo);
            }
        }
    }

    let mut new_tokens = Vec::new();
    let mut transactions = Vec::new();
    let timestamp = ic_cdk::api::time();
    let mut seen_hashes = std::collections::HashSet::new();

    for (i, mint_request) in req.mint_requests.iter().enumerate() {
        let token_id = current_token_id.clone() + Nat::from(i as u64);

        if let Some(pc) = &mint_request.private_content {
            for (entry_name, entry_hash) in &pc.entries {
                // Prevent double-spending a cache entry in the same batch
                if !seen_hashes.insert(*entry_hash) {
                    return Err(management::mint::MintError::StorageCanisterError(format!(
                        "Duplicate private content hash '{}' in batch",
                        hex::encode(entry_hash)
                    )));
                }

                let entry = read_state(|state| {
                    state
                        .data
                        .private_content_system
                        .temp_file_cache
                        .get(entry_hash)
                        .cloned()
                })
                .ok_or_else(|| {
                    management::mint::MintError::StorageCanisterError(format!(
                        "Private entry '{}' with hash '{}' not found in premint cache",
                        entry_name,
                        hex::encode(entry_hash)
                    ))
                })?;

                if entry.status != PrivateContentStatus::PendingMinting
                    && entry.status != PrivateContentStatus::Active
                {
                    return Err(management::mint::MintError::StorageCanisterError(format!(
                        "Private entry '{}' is not in a valid state for minting",
                        entry_name
                    )));
                }

                if entry.plaintext_size > MAX_PRIVATE_CONTENT_SIZE {
                    return Err(management::mint::MintError::StorageCanisterError(format!(
                        "Private entry '{}' exceeds maximum size",
                        entry_name
                    )));
                }

                // Verify identity consistency: content must be encrypted for the actual owner
                let expected_identity = construct_canonical_identity(
                    mint_request.token_owner.owner,
                    &entry.readers,
                    &pc.default_readers,
                );

                ic_cdk::println!(
                    "[mint] Validating private content identity for entry '{}'",
                    entry_name
                );
                ic_cdk::println!("[mint] Token owner: {}", mint_request.token_owner.owner);
                ic_cdk::println!("[mint] Entry readers: {:?}", entry.readers);
                ic_cdk::println!("[mint] Default readers: {:?}", pc.default_readers);
                ic_cdk::println!(
                    "[mint] Expected canonical identity: {:?}",
                    expected_identity
                );
                ic_cdk::println!(
                    "[mint] Stored canonical identity: {:?}",
                    entry.canonical_identity
                );
                if entry.canonical_identity != expected_identity {
                    return Err(management::mint::MintError::StorageCanisterError(format!(
                        "Private content identity mismatch for entry '{}'. It may have been initialized for a different owner.", entry_name
                    )));
                }
            }
        }

        // Validate public content up front (same rules as append_file): every
        // referenced file must be an already-finalized public upload. Registration
        // happens in the atomic mutate block below via `mint_public_content`.
        if let Some(pub_c) = &mint_request.public_content {
            for (entry_name, file_path) in &pub_c.entries {
                let entry_exists = read_state(|state| {
                    state
                        .data
                        .public_content_system
                        .temp_file_cache
                        .contains_key(file_path)
                        || state
                            .data
                            .public_content_system
                            .nft_public
                            .values()
                            .any(|record| record.entries.values().any(|e| &e.hash == file_path))
                });
                if !entry_exists {
                    return Err(management::mint::MintError::StorageCanisterError(format!(
                        "Public entry '{}' with path '{}' not found",
                        entry_name, file_path
                    )));
                }

                let is_finalized = read_state(|state| {
                    match state
                        .data
                        .public_content_system
                        .temp_file_cache
                        .get(file_path)
                    {
                        Some(entry) => entry.state == UploadState::Finalized,
                        None => true,
                    }
                });
                if !is_finalized {
                    return Err(management::mint::MintError::StorageCanisterError(format!(
                        "Public entry '{}' is not finalized",
                        entry_name
                    )));
                }
            }
        }

        let mut new_token = Icrc7Token::new(token_id.clone(), mint_request.token_owner.clone());
        __METADATA.with_borrow_mut(|m| new_token.add_metadata(m, mint_request.metadata.clone()));

        let transaction = ICRC7Transaction::new(
            "7mint".to_string(),
            timestamp,
            ICRC7TransactionData {
                op: "7mint".to_string(),
                tid: Some(token_id.clone()),
                from: None,
                to: Some(mint_request.token_owner.clone()),
                meta: None,
                memo: mint_request.memo.clone(),
                created_at_time: Some(Nat::from(timestamp)),
            },
        );

        new_tokens.push((token_id, new_token, mint_request.token_owner.clone()));
        transactions.push(transaction);
    }

    // Atomic block: If anything fails here, we should have trapped earlier or we use Results carefully.
    // On ICP, the ledger (icrc3_add_transaction) and tokens_list must be updated together.
    mutate_state(|state| {
        for (i, transaction) in transactions.into_iter().enumerate() {
            if let Err(e) = icrc3_add_transaction(transaction) {
                ic_cdk::trap(&format!(
                    "Atomic ledger update failed at index {}: {}",
                    i, e
                ));
            }
        }

        for (token_id, new_token, token_owner) in new_tokens.into_iter() {
            state.data.tokens_list.insert(token_id.clone(), new_token);
            state
                .data
                .tokens_list_by_owner
                .entry(token_owner)
                .or_insert(vec![])
                .push(token_id);
        }

        // Handle PendingMinting cache migrations
        for (i, mint_request) in req.mint_requests.iter().enumerate() {
            if let Some(pc) = &mint_request.private_content {
                let tid = current_token_id.clone() + Nat::from(i as u64);
                for entry_hash in pc.entries.values() {
                    if let Some(entry) = state
                        .data
                        .private_content_system
                        .temp_file_cache
                        .get(entry_hash)
                    {
                        state
                            .data
                            .public_content_system
                            .temp_file_cache
                            .remove(&entry.storage_path);
                    }
                }
                state
                    .data
                    .private_content_system
                    .mint_private_content(
                        pc.default_readers.clone(),
                        pc.entries.clone(),
                        tid.clone(),
                        mint_request.token_owner.owner,
                    )
                    .unwrap_or_else(|e| {
                        ic_cdk::trap(&format!(
                            "Failed to register private content for token {}: {:?}",
                            tid, e
                        ));
                    });
            }

            // Attach public content at mint (validated above). Moves each
            // finalized file from the public temp cache onto the new token.
            if let Some(pub_c) = &mint_request.public_content {
                let tid = current_token_id.clone() + Nat::from(i as u64);
                state
                    .data
                    .public_content_system
                    .mint_public_content(pub_c.entries.clone(), tid.clone())
                    .unwrap_or_else(|e| {
                        ic_cdk::trap(&format!(
                            "Failed to register public content for token {}: {:?}",
                            tid, e
                        ));
                    });
            }
        }

        state.data.last_token_id =
            current_token_id.clone() + Nat::from(req.mint_requests.len() as u64);
    });

    trace(&format!(
        "Successfully minted {} NFTs",
        req.mint_requests.len()
    ));
    Ok(current_token_id.clone())
}

#[update(guard = "caller_has_update_metadata_permission")]
pub fn append_file(req: management::append_file::Args) -> management::append_file::Response {
    trace("Appending files to NFT");
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller)
        .map_err(|_| management::append_file::AppendFileError::ConcurrentManagementCall)?;

    for request in &req.append_file_requests {
        // Validate that token exists
        let token_exists =
            read_state(|state| state.data.tokens_list.contains_key(&request.token_id));
        if !token_exists {
            return Err(management::append_file::AppendFileError::TokenDoesNotExists);
        }

        // Validate public content if present
        if let Some(pub_c) = &request.public_content {
            // Check for duplicate entry name
            let has_duplicate_entry = read_state(|state| {
                state
                    .data
                    .public_content_system
                    .nft_public
                    .get(&request.token_id)
                    .map(|record| {
                        pub_c
                            .entries
                            .keys()
                            .any(|entry_name| record.entries.contains_key(entry_name))
                    })
                    .unwrap_or(false)
            });
            if has_duplicate_entry {
                return Err(management::append_file::AppendFileError::FileAlreadyExists);
            }

            for (_entry_name, file_path) in &pub_c.entries {
                let entry_exists = read_state(|state| {
                    state
                        .data
                        .public_content_system
                        .temp_file_cache
                        .contains_key(file_path)
                        || state
                            .data
                            .public_content_system
                            .nft_public
                            .values()
                            .any(|record| record.entries.values().any(|e| &e.hash == file_path))
                });
                if !entry_exists {
                    return Err(management::append_file::AppendFileError::NotFound);
                }

                let is_finalized = read_state(|state| {
                    if let Some(entry) = state
                        .data
                        .public_content_system
                        .temp_file_cache
                        .get(file_path)
                    {
                        entry.state == UploadState::Finalized
                    } else {
                        true
                    }
                });
                if !is_finalized {
                    return Err(management::append_file::AppendFileError::InvalidStateTransition);
                }
            }
        }
    }

    mutate_state(|state| {
        for request in req.append_file_requests {
            // Append public content
            if let Some(pub_c) = request.public_content {
                let public_system = &mut state.data.public_content_system;

                let mut resolved_entries = Vec::new();
                for (entry_name, file_path) in pub_c.entries {
                    let mut entry = match public_system.temp_file_cache.remove(&file_path) {
                        Some(cached_entry) => cached_entry,
                        None => public_system
                            .nft_public
                            .values()
                            .find_map(|record| {
                                record.entries.values().find(|e| e.hash == file_path)
                            })
                            .cloned()
                            .unwrap(), // Safe because of prior validation
                    };
                    entry.state = UploadState::Finalized;
                    resolved_entries.push((entry_name, entry));
                }

                let public_record = public_system
                    .nft_public
                    .entry(request.token_id.clone())
                    .or_default();
                for (entry_name, entry) in resolved_entries {
                    let file_identity = entry.storage_path.clone();

                    public_system
                        .file_to_nfts
                        .entry(file_identity.clone())
                        .or_default()
                        .insert(request.token_id.clone());

                    public_system
                        .nft_to_files
                        .entry(request.token_id.clone())
                        .or_default()
                        .insert(file_identity);

                    public_record.entries.insert(entry_name, entry);
                }
            }
        }
    });

    Ok(())
}

#[update(guard = "caller_has_update_metadata_permission")]
pub fn update_nft_metadata(
    req: management::update_nft_metadata::Args,
) -> management::update_nft_metadata::Response {
    trace("Updating NFT metadata");
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|_| {
        management::update_nft_metadata::UpdateNftMetadataError::ConcurrentManagementCall
    })?;

    let token_name_hash = req.token_id;

    let token_list = read_state(|state| state.data.tokens_list.clone());

    match token_list.contains_key(&token_name_hash.clone()) {
        true => {
            let mut token = token_list.get(&token_name_hash.clone()).unwrap().clone();

            let previous_metadata =
                __METADATA.with_borrow(|m| m.get_all_data(Some(token_name_hash.clone())));

            __METADATA.with_borrow_mut(|m| token.replace_metadata(m, req.metadata));

            let new_metadata =
                __METADATA.with_borrow(|m| m.get_all_data(Some(token_name_hash.clone())));

            let mut metadata_map = BTreeMap::new();

            metadata_map.insert(
                "icrc7:previous_metadata".to_string(),
                Icrc3Value::Map(
                    previous_metadata
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(k, v)| (k, v.0))
                        .collect(),
                ),
            );

            metadata_map.insert(
                "icrc7:new_metadata".to_string(),
                Icrc3Value::Map(
                    new_metadata
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(k, v)| (k, v.0))
                        .collect(),
                ),
            );

            let meta = Icrc3Value::Map(metadata_map);

            let transaction = ICRC7Transaction::new(
                "7update_token".to_string(),
                ic_cdk::api::time(),
                ICRC7TransactionData {
                    op: "7update_token".to_string(),
                    tid: Some(token_name_hash.clone()),
                    from: Some(Account {
                        owner: ic_cdk::api::msg_caller(),
                        subaccount: None,
                    }),
                    to: None,
                    meta: Some(meta),
                    memo: None,
                    created_at_time: Some(Nat::from(ic_cdk::api::time())),
                },
            );

            match icrc3_add_transaction(transaction) {
                Ok(_) => {}
                Err(e) => {
                    return Err(management::update_nft_metadata::UpdateNftMetadataError::StorageCanisterError(
                        e.to_string(),
                    ));
                }
            };

            mutate_state(|state| {
                state
                    .data
                    .tokens_list
                    .insert(token_name_hash.clone(), token);
            });
            trace(&format!(
                "Updated NFT metadata for token: {:?}",
                token_name_hash.clone()
            ));
        }
        false => {
            return Err(management::update_nft_metadata::UpdateNftMetadataError::TokenDoesNotExist);
        }
    }

    Ok(token_name_hash.clone())
}

#[update]
pub async fn burn_nft(token_id: Nat) -> management::burn_nft::Response {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller)
        .map_err(|_| management::burn_nft::BurnNftError::ConcurrentManagementCall)?;

    let token = match read_state(|state| state.data.tokens_list.get(&token_id).cloned()) {
        Some(token) => token,
        None => {
            return Err(management::burn_nft::BurnNftError::TokenDoesNotExist);
        }
    };

    if caller != token.token_owner.owner {
        return Err(management::burn_nft::BurnNftError::NotTokenOwner);
    }

    let transaction = ICRC7Transaction::new(
        "7burn".to_string(),
        ic_cdk::api::time(),
        ICRC7TransactionData {
            op: "7burn".to_string(),
            tid: Some(token_id.clone()),
            from: Some(token.token_owner.clone()),
            to: None,
            meta: None,
            memo: None,
            created_at_time: Some(Nat::from(ic_cdk::api::time())),
        },
    );

    match icrc3_add_transaction(transaction) {
        Ok(_) => {}
        Err(e) => {
            return Err(management::burn_nft::BurnNftError::StorageCanisterError(
                e.to_string(),
            ));
        }
    }

    let private_files = read_state(|state| {
        let mut files = Vec::new();
        if let Some(private_record) = state.data.private_content_system.nft_private.get(&token_id) {
            for entry in private_record.entries.values() {
                files.push((entry.storage_canister_id, entry.storage_path.clone()));
            }
        }
        files
    });

    let mut sub_canister_manager = read_state(|state| state.data.sub_canister_manager.clone());
    for (canister_id, storage_path) in private_files {
        if let Some(canister) = sub_canister_manager.get_canister(canister_id) {
            let remove_arg = bity_ic_storage_canister_api::updates::remove_file::Args {
                file_path: storage_path,
            };
            let _ = canister.remove_file(remove_arg).await;
        }
    }

    mutate_state(|state| {
        state.data.tokens_list.remove(&token_id);
        state
            .data
            .private_content_system
            .nft_private
            .remove(&token_id);
    });

    Ok(())
}

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn init_upload(data: core_nft_api::InitUploadArgs) -> core_nft_api::InitUploadResponse {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller)
        .map_err(|_| management::init_upload::InitUploadError::ConcurrentManagementCall)?;

    let file_exists = read_state(|state| state.internal_filestorage.contains_path(&data.file_path));
    if file_exists {
        return Err(management::init_upload::InitUploadError::FileAlreadyExists);
    }

    read_state(|state| {
        state
            .data
            .public_content_system
            .init_upload_validate(&data.file_path, data.file_size)
    })?;

    let timestamp = ic_cdk::api::time();
    let mut sub_canister_manager = read_state(|state| state.data.sub_canister_manager.clone());

    let canister = match sub_canister_manager.init_upload(data.clone()).await {
        Ok((_, canister)) => canister,
        Err(e) => {
            trace(&format!("Error inserting data: {:?}", e));
            return Err(management::init_upload::InitUploadError::StorageCanisterError(e));
        }
    };

    // FIXME
    let chunk_size = data.chunk_size.unwrap_or(CHUNK_SIZE.try_into().unwrap());
    let expected_chunks = data.file_size.div_ceil(chunk_size).try_into().unwrap();
    mutate_state(|state| {
        state.data.sub_canister_manager = sub_canister_manager;
        let register_res = state.data.public_content_system.init_upload_register(
            data.file_path.clone(),
            canister,
            data.file_path.clone(),
            expected_chunks,
            data.file_size,
            timestamp,
        );
        if register_res.is_ok() {
            state.internal_filestorage.insert(
                data.file_path.clone(),
                crate::state::InternalFilestorageData {
                    init_timestamp: timestamp,
                    state: UploadState::InProgress,
                    canister,
                    path: data.file_path,
                },
            );
        }
        register_res
    })
}

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn store_chunk(data: management::store_chunk::Args) -> management::store_chunk::Response {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller)
        .map_err(|_| management::store_chunk::StoreChunkError::ConcurrentManagementCall)?;

    let (_init_timestamp, canister_id, _file_path) = match read_state(|state| {
        state
            .data
            .public_content_system
            .get_public_file_by_path(&data.file_path)
    }) {
        Some(data) => match data.state {
            UploadState::Init
            | UploadState::ReuploadInit
            | UploadState::InProgress
            | UploadState::InitReupload
            | UploadState::ChunkReupload
            | UploadState::FinalizeReupload => (
                data.created_at_ns,
                data.storage_canister_id,
                data.storage_path,
            ),
            UploadState::Finalized => {
                return Err(management::store_chunk::StoreChunkError::UploadAlreadyFinalized);
            }
        },
        None => {
            return Err(management::store_chunk::StoreChunkError::UploadNotInitialized);
        }
    };

    let canister: StorageCanister = match read_state(|state| {
        state
            .data
            .sub_canister_manager
            .get_canister(canister_id.clone())
    }) {
        Some(canister) => canister,
        None => {
            mutate_state(|state| {
                state
                    .data
                    .public_content_system
                    .temp_file_cache
                    .remove(&data.file_path);
            });
            return Err(
                management::store_chunk::StoreChunkError::StorageCanisterError(
                    "Storage canister not found. Cancelling the upload.".to_string(),
                ),
            );
        }
    };

    match canister.store_chunk(data.clone()).await {
        Ok(_) => {}
        Err(e) => {
            trace(&format!("Error storing chunk: {:?}", e));
            return Err(e);
        }
    }

    mutate_state(|state| {
        state.data.public_content_system.upload_chunk_register(
            &data.file_path,
            data.chunk_id,
            data.chunk_data,
        )
    })?;

    Ok(management::store_chunk::StoreChunkResp {})
}

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn finalize_upload(data: finalize_upload::Args) -> finalize_upload::Response {
    trace(&format!("Finalizing upload: {:?}", data));
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller)
        .map_err(|_| finalize_upload::FinalizeUploadError::ConcurrentManagementCall)?;

    let (init_timestamp, media_path, canister_id) = match read_state(|state| {
        state
            .data
            .public_content_system
            .get_public_file_by_path(&data.file_path)
    }) {
        Some(data) => match data.state {
            UploadState::Init | UploadState::ReuploadInit | UploadState::InitReupload => {
                return Err(finalize_upload::FinalizeUploadError::UploadNotStarted);
            }
            UploadState::InProgress
            | UploadState::ChunkReupload
            | UploadState::FinalizeReupload => (
                data.created_at_ns,
                data.storage_path,
                data.storage_canister_id,
            ),
            UploadState::Finalized => {
                return Err(finalize_upload::FinalizeUploadError::UploadAlreadyFinalized);
            }
        },
        None => {
            return Err(finalize_upload::FinalizeUploadError::UploadNotStarted);
        }
    };

    let canister: StorageCanister = match read_state(|state| {
        state
            .data
            .sub_canister_manager
            .get_canister(canister_id.clone())
    }) {
        Some(canister) => canister,
        None => {
            mutate_state(|state| {
                state
                    .data
                    .public_content_system
                    .temp_file_cache
                    .remove(&data.file_path);
            });
            return Err(finalize_upload::FinalizeUploadError::StorageCanisterError(
                "Storage canister not found. Cancelling the upload.".to_string(),
            ));
        }
    };

    match canister.finalize_upload(data.clone()).await {
        Ok(_) => {}
        Err(e) => {
            trace(&format!("Error storing chunk: {:?}", e));
            // TODO shall we automaticly cleanup or add a cleanup and let user retry?
            return Err(e);
        }
    }

    let path = if media_path.starts_with('/') {
        media_path.clone()
    } else {
        format!("/{}", media_path)
    };

    // Construct URLs based on base_url configuration or test_mode
    // base_url should be a template with {canister_id} placeholder, e.g.:
    //   - "http://{canister_id}.localhost:4943" (local)
    //   - "https://{canister_id}.raw.icp0.io" (mainnet)
    let (base_url_template, is_test_mode) =
        read_state(|state| (state.data.base_url.clone(), state.env.is_test_mode()));

    let construct_url = |canister_id_str: &str, file_path: &str| -> String {
        match &base_url_template {
            Some(template) => {
                let base = template.replace("{canister_id}", canister_id_str);
                format!("{}{}", base.trim_end_matches('/'), file_path)
            }
            None => {
                if is_test_mode {
                    format!("http://{}.localhost:4943{}", canister_id_str, file_path)
                } else {
                    format!("https://{}.raw.icp0.io{}", canister_id_str, file_path)
                }
            }
        }
    };

    let redirection_url = construct_url(&canister_id.to_string(), &path);

    add_redirection(path.clone(), redirection_url.clone());

    mutate_state(|state| {
        let _ = state
            .data
            .public_content_system
            .finalize_upload_register(&data.file_path);

        state
            .data
            .media_redirections
            .insert(path.clone(), redirection_url);

        if let Some(entry) = state.internal_filestorage.map.get_mut(&data.file_path) {
            entry.state = UploadState::Finalized;
        } else {
            state.internal_filestorage.insert(
                data.file_path.clone(),
                crate::state::InternalFilestorageData {
                    init_timestamp: init_timestamp,
                    state: UploadState::Finalized,
                    canister: canister_id,
                    path: media_path,
                },
            );
        }
    });

    let main_canister_id = ic_cdk::api::canister_self().to_string();
    let url = construct_url(&main_canister_id, &path);

    Ok(finalize_upload::FinalizeUploadResp { url })
}

#[query(guard = "caller_has_manage_authorities_permission")]
pub fn get_all_storage_subcanisters() -> Vec<candid::Principal> {
    read_state(|state| state.data.sub_canister_manager.list_canisters_ids())
}

#[query(guard = "caller_has_read_uploads_permission")]
pub fn get_upload_status(file_path: String) -> management::get_upload_status::Response {
    let upload_status = read_state(|state| {
        if let Some(status) = state
            .data
            .public_content_system
            .get_public_file_by_path(&file_path)
        {
            ic_cdk::println!(
                "get_upload_status: found in public_content_system: {:?}",
                status.state
            );
            return Some(status.state);
        }
        for entry in state.data.private_content_system.temp_file_cache.values() {
            if entry.storage_path == file_path {
                ic_cdk::println!(
                    "get_upload_status: found in temp_file_cache: status={:?}, pending={:?}",
                    entry.status,
                    entry.pending_upload.is_some()
                );
                if let Some(pending) = &entry.pending_upload {
                    if pending.received_chunks.is_empty() {
                        return Some(UploadState::Init);
                    } else {
                        return Some(UploadState::InProgress);
                    }
                } else {
                    return Some(UploadState::Finalized);
                }
            }
        }
        for record in state.data.private_content_system.nft_private.values() {
            for entry in record.entries.values() {
                if entry.storage_path == file_path {
                    ic_cdk::println!(
                        "get_upload_status: found in nft_private: status={:?}, pending={:?}",
                        entry.status,
                        entry.pending_upload.is_some()
                    );
                    if let Some(pending) = &entry.pending_upload {
                        if pending.received_chunks.is_empty() {
                            return Some(UploadState::InitReupload);
                        } else {
                            return Some(UploadState::ChunkReupload);
                        }
                    } else {
                        return Some(UploadState::Finalized);
                    }
                }
            }
        }
        None
    });
    match upload_status {
        Some(state) => Ok(state),
        None => Err(management::get_upload_status::GetUploadStatusError::UploadNotFound),
    }
}

#[query(guard = "caller_has_read_uploads_permission")]
pub fn get_all_uploads(
    prev: Option<Nat>,
    take: Option<Nat>,
) -> management::get_all_uploads::Response {
    trace(&format!("prev: {:?}, take: {:?}", prev, take));
    let uploads = read_state(|state| {
        state
            .data
            .public_content_system
            .get_paginated_uploads(prev, take)
    });

    Ok(uploads)
}

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn cancel_upload(data: cancel_upload::Args) -> cancel_upload::Response {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller)
        .map_err(|_| cancel_upload::CancelUploadError::ConcurrentManagementCall)?;

    let (canister_id, key_to_remove) = match read_state(|state| {
        state
            .data
            .public_content_system
            .get_public_file_by_path(&data.file_path)
    }) {
        Some(entry) => match entry.state {
            UploadState::Init
            | UploadState::ReuploadInit
            | UploadState::InProgress
            | UploadState::InitReupload
            | UploadState::ChunkReupload
            | UploadState::FinalizeReupload => (entry.storage_canister_id, entry.storage_path),
            UploadState::Finalized => {
                return Err(cancel_upload::CancelUploadError::UploadAlreadyFinalized);
            }
        },
        None => {
            return Err(cancel_upload::CancelUploadError::UploadNotInitialized);
        }
    };

    let canister: StorageCanister = match read_state(|state| {
        state
            .data
            .sub_canister_manager
            .get_canister(canister_id.clone())
    }) {
        Some(canister) => canister,
        None => {
            mutate_state(|state| {
                state
                    .data
                    .public_content_system
                    .cancel_upload_register(&key_to_remove);
                state.internal_filestorage.remove(&data.file_path);
            });
            return Err(cancel_upload::CancelUploadError::StorageCanisterError(
                "Storage canister not found. Cancelling the upload.".to_string(),
            ));
        }
    };

    match canister.cancel_upload(data.clone()).await {
        Ok(_) => {}
        Err(e) => {
            trace(&format!("Error storing chunk: {:?}", e));
            return Err(e);
        }
    }

    mutate_state(|state| {
        state
            .data
            .public_content_system
            .cancel_upload_register(&key_to_remove);
        state.internal_filestorage.remove(&data.file_path);
    });

    Ok(cancel_upload::CancelUploadResp {})
}

#[update(guard = "caller_has_manage_authorities_permission")]
pub fn grant_permission(args: grant_permission::Args) -> grant_permission::Response {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller)
        .map_err(|_| grant_permission::GrantPermissionError::ConcurrentManagementCall)?;

    mutate_state(|state| {
        state
            .data
            .permissions
            .grant_permission(args.principal, args.permission);
    });

    Ok(())
}

#[update(guard = "caller_has_manage_authorities_permission")]
pub fn revoke_permission(args: revoke_permission::Args) -> revoke_permission::Response {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller)
        .map_err(|_| revoke_permission::RevokePermissionError::ConcurrentManagementCall)?;

    mutate_state(|state| {
        state
            .data
            .permissions
            .revoke_permission(&args.principal, &args.permission);
    });

    Ok(())
}

#[query(guard = "caller_has_manage_authorities_permission")]
pub fn get_user_permissions(args: get_user_permissions::Args) -> get_user_permissions::Response {
    let permissions = read_state(|state| {
        state
            .data
            .permissions
            .get_permissions(&args.principal)
            .cloned()
    });
    match permissions {
        Some(permissions) => Ok(permissions),
        None => Err(get_user_permissions::GetUserPermissionsError::UserNotFound),
    }
}

#[query(guard = "caller_has_manage_authorities_permission")]
pub fn has_permission(args: has_permission::Args) -> has_permission::Response {
    let has_permission = read_state(|state| {
        state
            .data
            .permissions
            .has_permission(&args.principal, &args.permission)
    });

    Ok(has_permission)
}

#[cfg(feature = "inttest")]
#[query]
pub fn __get_public_entry_test(
    args: __get_public_entry_test::Args,
) -> __get_public_entry_test::Response {
    read_state(|state| {
        state
            .data
            .public_content_system
            .get_nft_public_entry(args.token_id, &args.entry_name)
            .map_err(|e| e.to_string())
    })
}
