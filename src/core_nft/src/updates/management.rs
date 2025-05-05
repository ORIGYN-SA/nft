use crate::guards::{caller_is_governance_principal, GuardManagement};
use crate::state::{icrc3_add_transaction, mutate_state, read_state, InternalFilestorageData};
use crate::types::http::{add_redirection, remove_redirection};
use crate::types::sub_canister::StorageCanister;
use crate::types::transaction::{Icrc3Transaction, TransactionData};
use crate::types::{icrc7, management, nft};
use crate::utils::{check_memo, hash_string_to_u64, trace};
pub use candid::Nat;
pub use ic_cdk::api::call::RejectionCode;
use ic_cdk_macros::update;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use icrc_ledger_types::icrc1::account::Account;
use std::collections::BTreeMap;
pub use storage_api_canister::cancel_upload;
pub use storage_api_canister::delete_file;
pub use storage_api_canister::finalize_upload;
pub use storage_api_canister::init_upload;
pub use storage_api_canister::store_chunk;

//TODO Use minting autority to mint tokens
#[update(guard = "caller_is_governance_principal")]
pub async fn mint(req: management::mint::Args) -> management::mint::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let token_name_hash = Nat::from(hash_string_to_u64(&req.token_name));

    let token_list = read_state(|state| state.data.tokens_list.clone());
    let supply_cap = read_state(|state| {
        state
            .data
            .supply_cap
            .clone()
            .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_SUPPLY_CAP))
    });

    if token_list.len() >= supply_cap {
        return Err((
            RejectionCode::CanisterError,
            "Exceed Max allowed Supply Cap".to_string(),
        ));
    }

    match check_memo(req.memo.clone()) {
        Ok(_) => {}
        Err(e) => {
            return Err((RejectionCode::CanisterError, e));
        }
    }

    match token_list.contains_key(&token_name_hash.clone()) {
        true => {
            return Err((
                RejectionCode::CanisterError,
                "Token already exists".to_string(),
            ));
        }
        false => {
            let new_token = nft::Icrc7Token::new(
                token_name_hash.clone(),
                req.token_name,
                req.token_description,
                req.token_logo,
                req.token_owner,
            );

            let transaction = Icrc3Transaction {
                btype: "7mint".to_string(),
                timestamp: ic_cdk::api::time(),
                tx: TransactionData {
                    tid: Some(token_name_hash.clone()),
                    from: None,
                    to: Some(req.token_owner.clone()),
                    meta: None,
                    memo: req.memo.clone(),
                    created_at_time: Some(Nat::from(ic_cdk::api::time())),
                    spender: None,
                    exp: None,
                },
            };

            match icrc3_add_transaction(transaction).await {
                Ok(_) => {}
                Err(e) => {
                    return Err((
                        RejectionCode::CanisterError,
                        format!("Failed to log transaction: {}", e),
                    ));
                }
            }

            mutate_state(|state| {
                state
                    .data
                    .tokens_list
                    .insert(token_name_hash.clone(), new_token);
            });
        }
    }

    Ok(token_name_hash.clone())
}

#[update(guard = "caller_is_governance_principal")]
pub async fn update_nft_metadata(
    req: management::update_nft_metadata::Args,
) -> management::update_nft_metadata::Response {
    trace("Updating NFT metadata");
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let token_name_hash = req.token_id;

    let token_list = read_state(|state| state.data.tokens_list.clone());

    match token_list.contains_key(&token_name_hash.clone()) {
        true => {
            let mut token = token_list.get(&token_name_hash.clone()).unwrap().clone();
            let mut metadata_map = BTreeMap::new();
            if let Some(name) = req.token_name {
                token.token_name = name.clone();
                metadata_map.insert(
                    "icrc7:token_name".to_string(),
                    Icrc3Value::Text(name.clone()),
                );
            }
            if let Some(description) = req.token_description {
                token.token_description = Some(description.clone());
                metadata_map.insert(
                    "icrc7:token_description".to_string(),
                    Icrc3Value::Text(description.clone()),
                );
            }
            if let Some(logo) = req.token_logo {
                token.token_logo = Some(logo.clone());
                metadata_map.insert(
                    "icrc7:token_logo".to_string(),
                    Icrc3Value::Text(logo.clone()),
                );
            }
            if let Some(metadata) = req.token_metadata {
                trace(&format!("Adding metadata to token: {:?}", metadata));
                token.add_metadata(metadata.clone()).await;
                let mut btree_metadata = BTreeMap::new();
                for (key, value) in metadata {
                    btree_metadata.insert(key.clone(), value.clone());
                }
                metadata_map.insert(
                    "icrc7:token_metadata".to_string(),
                    Icrc3Value::Map(btree_metadata),
                );
            }

            let transaction = Icrc3Transaction {
                btype: "7update_token".to_string(),
                timestamp: ic_cdk::api::time(),
                tx: TransactionData {
                    tid: Some(token_name_hash.clone()),
                    from: Some(Account {
                        owner: ic_cdk::caller(),
                        subaccount: None,
                    }),
                    to: None,
                    meta: Some(Icrc3Value::Map(metadata_map)),
                    memo: None,
                    created_at_time: Some(Nat::from(ic_cdk::api::time())),
                    spender: None,
                    exp: None,
                },
            };

            match icrc3_add_transaction(transaction).await {
                Ok(_) => {}
                Err(e) => {
                    return Err((
                        RejectionCode::CanisterError,
                        format!("Failed to log transaction: {}", e),
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
            return Err((
                RejectionCode::CanisterError,
                "Token does not exist".to_string(),
            ));
        }
    }

    // TODO add transactions logs

    Ok(token_name_hash.clone())
}

#[update(guard = "caller_is_governance_principal")]
pub async fn burn_nft(token_id: Nat) -> Result<(), (RejectionCode, String)> {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let token = match read_state(|state| state.data.tokens_list.get(&token_id).cloned()) {
        Some(token) => token,
        None => {
            return Err((
                RejectionCode::CanisterError,
                "Token does not exist".to_string(),
            ));
        }
    };

    let transaction = Icrc3Transaction {
        btype: "7burn".to_string(),
        timestamp: ic_cdk::api::time(),
        tx: TransactionData {
            tid: Some(token_id.clone()),
            from: Some(token.token_owner.clone()),
            to: None,
            meta: None,
            memo: None,
            created_at_time: Some(Nat::from(ic_cdk::api::time())),
            spender: None,
            exp: None,
        },
    };

    match icrc3_add_transaction(transaction).await {
        Ok(_) => {}
        Err(e) => {
            return Err((
                RejectionCode::CanisterError,
                format!("Failed to log transaction: {}", e),
            ));
        }
    }

    mutate_state(|state| {
        state.data.tokens_list.remove(&token_id);
    });

    Ok(())
}

#[update(guard = "caller_is_governance_principal")]
pub fn update_minting_authorities(
    req: management::update_minting_authorities::Args,
) -> management::update_minting_authorities::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let mut minting_authorities = req.minting_authorities.clone();
    let previous_minting_authorities = mutate_state(|state| state.data.minting_authorities.clone());

    minting_authorities.append(&mut previous_minting_authorities.clone());
    minting_authorities.sort();
    minting_authorities.dedup();

    mutate_state(|state| {
        state.data.minting_authorities = minting_authorities.into_iter().collect();
    });

    Ok(())
}

#[update(guard = "caller_is_governance_principal")]
pub fn remove_minting_authorities(
    req: management::remove_minting_authorities::Args,
) -> management::remove_minting_authorities::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let mut minting_authorities = req.minting_authorities.clone();
    let previous_minting_authorities = mutate_state(|state| state.data.minting_authorities.clone());

    minting_authorities.retain(|auth| !previous_minting_authorities.contains(auth));

    mutate_state(|state| {
        state.data.minting_authorities = minting_authorities.into_iter().collect();
    });

    Ok(())
}

#[update(guard = "caller_is_governance_principal")]
pub async fn init_upload(data: init_upload::Args) -> init_upload::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    if read_state(|state| state.internal_filestorage.contains_path(&data.file_path)) {
        return Err((RejectionCode::CanisterError, "File exists.".to_string()));
    }

    let mut sub_canister_manager = read_state(|state| state.data.sub_canister_manager.clone());

    let canister = match sub_canister_manager.init_upload(data.clone()).await {
        Ok((_, canister)) => canister,
        Err(e) => {
            trace(&format!("Error inserting data: {:?}", e));
            return Err((RejectionCode::CanisterError, e));
        }
    };

    mutate_state(|state| {
        state.data.sub_canister_manager = sub_canister_manager;
        state.internal_filestorage.insert(
            data.file_path.clone(),
            InternalFilestorageData {
                init_timestamp: ic_cdk::api::time(),
                state: crate::state::UploadState::Init,
                canister: canister,
                path: data.file_path,
            },
        );
    });

    Ok(init_upload::InitUploadResp {})
}

#[update(guard = "caller_is_governance_principal")]
pub async fn store_chunk(data: store_chunk::Args) -> store_chunk::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let (init_timestamp, canister_id, file_path) =
        match read_state(|state| state.internal_filestorage.get(&data.file_path).cloned()) {
            Some(data) => match data.state {
                crate::state::UploadState::Init => (data.init_timestamp, data.canister, data.path),
                crate::state::UploadState::InProgress => {
                    (data.init_timestamp, data.canister, data.path)
                }
                crate::state::UploadState::Finalized => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Core - store_chunk - Upload already finalized".to_string(),
                    ));
                }
            },
            None => {
                return Err((
                    RejectionCode::CanisterError,
                    "Upload not initiated".to_string(),
                ));
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
                state.internal_filestorage.remove(&data.file_path);
            });
            return Err((
                RejectionCode::CanisterError,
                "Storage canister not found. Cancelling the upload.".to_string(),
            ));
        }
    };

    match canister.store_chunk(data.clone()).await {
        Ok(_) => {}
        Err(e) => {
            trace(&format!("Error storing chunk: {:?}", e));
            return Err((RejectionCode::CanisterError, e));
        }
    }
    mutate_state(|state| {
        state.internal_filestorage.insert(
            data.file_path.clone(),
            InternalFilestorageData {
                init_timestamp: init_timestamp,
                state: crate::state::UploadState::InProgress,
                canister: canister_id,
                path: file_path,
            },
        );
    });

    Ok(store_chunk::StoreChunkResp {})
}

#[update(guard = "caller_is_governance_principal")]
pub async fn finalize_upload(data: finalize_upload::Args) -> finalize_upload::Response {
    trace(&format!("Finalizing upload: {:?}", data));
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let (init_timestamp, media_path, canister_id) =
        match read_state(|state| state.internal_filestorage.get(&data.file_path).cloned()) {
            Some(data) => match data.state {
                crate::state::UploadState::Init => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Upload didnt started".to_string(),
                    ));
                }
                crate::state::UploadState::InProgress => {
                    (data.init_timestamp, data.path, data.canister)
                }
                crate::state::UploadState::Finalized => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Core - finalize_upload - Upload already finalized".to_string(),
                    ));
                }
            },
            None => {
                return Err((
                    RejectionCode::CanisterError,
                    "Upload not initiated".to_string(),
                ));
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
                state.internal_filestorage.remove(&data.file_path);
            });
            return Err((
                RejectionCode::CanisterError,
                "Storage canister not found. Cancelling the upload.".to_string(),
            ));
        }
    };

    match canister.finalize_upload(data.clone()).await {
        Ok(_) => {}
        Err(e) => {
            trace(&format!("Error storing chunk: {:?}", e));
            // TODO shall we automaticly cleanup or add a cleanup and let user retry?
            return Err((RejectionCode::CanisterError, e));
        }
    }

    let redirection_url = format!("https://{}.raw.icp0.io{}", canister_id, media_path.clone());

    add_redirection(media_path.clone(), redirection_url);

    mutate_state(|state| {
        state.internal_filestorage.insert(
            data.file_path.clone(),
            InternalFilestorageData {
                init_timestamp: init_timestamp,
                state: crate::state::UploadState::Finalized,
                canister: canister_id,
                path: media_path,
            },
        );
    });

    return Ok(finalize_upload::FinalizeUploadResp {});
}

#[update(guard = "caller_is_governance_principal")]
pub async fn cancel_upload(data: cancel_upload::Args) -> cancel_upload::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let (media_path, canister_id) =
        match read_state(|state| state.internal_filestorage.get(&data.file_path).cloned()) {
            Some(data) => match data.state {
                crate::state::UploadState::Init => (data.path, data.canister),
                crate::state::UploadState::InProgress => (data.path, data.canister),
                crate::state::UploadState::Finalized => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Core - cancel_upload - Upload already finalized".to_string(),
                    ));
                }
            },
            None => {
                return Err((
                    RejectionCode::CanisterError,
                    "Upload not initiated".to_string(),
                ));
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
                state.internal_filestorage.remove(&data.file_path);
            });
            return Err((
                RejectionCode::CanisterError,
                "Storage canister not found. Cancelling the upload.".to_string(),
            ));
        }
    };

    match canister.cancel_upload(data.clone()).await {
        Ok(_) => {}
        Err(e) => {
            trace(&format!("Error storing chunk: {:?}", e));
            return Err((RejectionCode::CanisterError, e));
        }
    }

    mutate_state(|state| {
        state.internal_filestorage.remove(&data.file_path);
    });

    Ok(cancel_upload::CancelUploadResp {})
}

#[update(guard = "caller_is_governance_principal")]
pub async fn delete_file(data: delete_file::Args) -> delete_file::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let (media_path, canister_id) =
        match read_state(|state| state.internal_filestorage.get(&data.file_path).cloned()) {
            Some(data) => match data.state {
                crate::state::UploadState::Init => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Upload didnt started".to_string(),
                    ));
                }
                crate::state::UploadState::InProgress => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Upload in progress".to_string(),
                    ));
                }
                crate::state::UploadState::Finalized => (data.path, data.canister),
            },
            None => {
                return Err((
                    RejectionCode::CanisterError,
                    "Upload not initiated".to_string(),
                ));
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
                state.internal_filestorage.remove(&data.file_path);
            });
            return Err((
                RejectionCode::CanisterError,
                "Storage canister not found. Cancelling the upload.".to_string(),
            ));
        }
    };

    match canister.delete_file(data.clone()).await {
        Ok(_) => {}
        Err(e) => {
            trace(&format!("Error storing chunk: {:?}", e));
            return Err((RejectionCode::CanisterError, e));
        }
    }

    mutate_state(|state| {
        state.internal_filestorage.remove(&data.file_path);
    });

    remove_redirection(
        media_path.clone(),
        format!("https://{}.raw.icp0.io{}", canister_id, media_path.clone()),
    );

    Ok(delete_file::DeleteFileResp {})
}
