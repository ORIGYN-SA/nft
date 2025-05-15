use crate::guards::{caller_is_governance_principal, caller_is_minting_authority, GuardManagement};
use crate::state::{icrc3_add_transaction, mutate_state, read_state, InternalFilestorageData};
use crate::types::http::{add_redirection, remove_redirection};
use crate::types::sub_canister::StorageCanister;
use crate::types::transaction::{Icrc3Transaction, TransactionData};
use crate::types::{icrc7, management, nft};
use crate::utils::{check_memo, trace};

pub use candid::Nat;
pub use ic_cdk::api::call::RejectionCode;
use ic_cdk_macros::{query, update};
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use icrc_ledger_types::icrc1::account::Account;
use std::collections::BTreeMap;
use std::collections::HashMap;
pub use storage_api_canister::cancel_upload;
pub use storage_api_canister::delete_file;
pub use storage_api_canister::finalize_upload;
pub use storage_api_canister::init_upload;
pub use storage_api_canister::store_chunk;
use storage_api_canister::types::storage::UploadState;

#[update(guard = "caller_is_governance_principal")]
pub async fn update_collection_metadata(
    req: management::update_collection_metadata::Args,
) -> Result<(), (RejectionCode, String)> {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

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

    if let Some(collection_metadata) = req.collection_metadata {
        mutate_state(|state| {
            state
                .data
                .metadata
                .erase_all_data(None, collection_metadata);
        });
    }

    Ok(())
}

#[update(guard = "caller_is_minting_authority")]
pub async fn mint(req: management::mint::Args) -> management::mint::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let token_id = read_state(|state| state.data.last_token_id.clone());

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

    match token_list.contains_key(&token_id.clone()) {
        true => {
            return Err((
                RejectionCode::CanisterError,
                "Token already exists".to_string(),
            ));
        }
        false => {
            let new_token = nft::Icrc7Token::new(
                token_id.clone(),
                req.token_name,
                req.token_description,
                req.token_logo,
                req.token_owner,
            );

            let transaction = Icrc3Transaction {
                btype: "7mint".to_string(),
                timestamp: ic_cdk::api::time(),
                tx: TransactionData {
                    tid: Some(token_id.clone()),
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
                state.data.last_token_id = token_id.clone() + Nat::from(1u64);
                state.data.tokens_list.insert(token_id.clone(), new_token);
            });
        }
    }

    Ok(token_id.clone())
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
            trace(&format!("token_list: true"));
            let mut token = token_list.get(&token_name_hash.clone()).unwrap().clone();
            let mut metadata_map = BTreeMap::new();
            if let Some(name) = req.token_name {
                trace(&format!("name: {:?}", name));
                token.token_name = name.clone();
                metadata_map.insert(
                    "icrc7:token_name".to_string(),
                    Icrc3Value::Text(name.clone()),
                );
            }
            if let Some(description) = req.token_description {
                trace(&format!("description: {:?}", description));
                token.token_description = Some(description.clone());
                metadata_map.insert(
                    "icrc7:token_description".to_string(),
                    Icrc3Value::Text(description.clone()),
                );
            }
            if let Some(logo) = req.token_logo {
                trace(&format!("logo: {:?}", logo));
                token.token_logo = Some(logo.clone());
                metadata_map.insert(
                    "icrc7:token_logo".to_string(),
                    Icrc3Value::Text(logo.clone()),
                );
            }
            if let Some(metadata) = req.token_metadata.clone() {
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

            if let Some(metadata) = req.token_metadata {
                trace(&format!("Adding metadata to token: {:?}", metadata));

                mutate_state(|state| {
                    token.add_metadata(&mut state.data.metadata, metadata.clone());
                });
            }

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
                state: UploadState::Init,
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
                UploadState::Init => (data.init_timestamp, data.canister, data.path),
                UploadState::InProgress => (data.init_timestamp, data.canister, data.path),
                UploadState::Finalized => {
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
                state: UploadState::InProgress,
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
                UploadState::Init => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Upload didnt started".to_string(),
                    ));
                }
                UploadState::InProgress => (data.init_timestamp, data.path, data.canister),
                UploadState::Finalized => {
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

    let redirection_url = format!("https://{}.raw.icp0.io/{}", canister_id, media_path.clone());

    let path = if media_path.starts_with('/') {
        media_path.clone()
    } else {
        format!("/{}", media_path)
    };

    add_redirection(path.clone(), redirection_url.clone());

    mutate_state(|state| {
        state
            .data
            .media_redirections
            .insert(path.clone(), redirection_url);

        state.internal_filestorage.insert(
            data.file_path.clone(),
            InternalFilestorageData {
                init_timestamp: init_timestamp,
                state: UploadState::Finalized,
                canister: canister_id,
                path: media_path,
            },
        );
    });

    return Ok(finalize_upload::FinalizeUploadResp {});
}

#[query]
pub fn get_upload_status(file_path: String) -> management::get_upload_status::Response {
    let upload_status = read_state(|state| state.internal_filestorage.get(&file_path).cloned());
    match upload_status {
        Some(status) => Ok(status.state),
        None => Err((RejectionCode::CanisterError, "Upload not found".to_string())),
    }
}

#[query]
pub fn get_all_uploads(
    prev: Option<Nat>,
    take: Option<Nat>,
) -> management::get_all_uploads::Response {
    trace(&format!("prev: {:?}, take: {:?}", prev, take));
    let all_uploads = read_state(|state| state.internal_filestorage.clone());
    let start: usize = usize::try_from(prev.unwrap_or(Nat::from(0u64)).0).unwrap_or(0);
    let end: usize = usize::try_from(take.unwrap_or(Nat::from(100u64)).0).unwrap_or(100);
    trace(&format!("start: {:?}, end: {:?}", start, end));
    let filtered_uploads: HashMap<String, UploadState> = all_uploads
        .map
        .iter()
        .skip(start)
        .take(end)
        .map(|(path, status)| (path.clone(), status.state.clone()))
        .collect();

    Ok(filtered_uploads)
}

#[update(guard = "caller_is_governance_principal")]
pub async fn cancel_upload(data: cancel_upload::Args) -> cancel_upload::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let (media_path, canister_id) =
        match read_state(|state| state.internal_filestorage.get(&data.file_path).cloned()) {
            Some(data) => match data.state {
                UploadState::Init => (data.path, data.canister),
                UploadState::InProgress => (data.path, data.canister),
                UploadState::Finalized => {
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
                UploadState::Init => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Upload didnt started".to_string(),
                    ));
                }
                UploadState::InProgress => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Upload in progress".to_string(),
                    ));
                }
                UploadState::Finalized => (data.path, data.canister),
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
pub async fn update_authorized_principals(
    req: management::update_authorized_principals::Args,
) -> management::update_authorized_principals::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let mut authorized_principals = req.authorized_principals.clone();
    let previous_authorized_principals =
        mutate_state(|state| state.data.authorized_principals.clone());

    authorized_principals.append(&mut previous_authorized_principals.clone());
    authorized_principals.sort();
    authorized_principals.dedup();

    mutate_state(|state| {
        state.data.authorized_principals = authorized_principals.into_iter().collect();
    });

    Ok(())
}

#[update(guard = "caller_is_governance_principal")]
pub async fn remove_authorized_principals(
    req: management::remove_authorized_principals::Args,
) -> management::remove_authorized_principals::Response {
    let caller = ic_cdk::caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| (RejectionCode::CanisterError, e))?;

    let mut authorized_principals = req.authorized_principals.clone();
    let previous_authorized_principals =
        mutate_state(|state| state.data.authorized_principals.clone());

    authorized_principals.retain(|auth| !previous_authorized_principals.contains(auth));

    mutate_state(|state| {
        state.data.authorized_principals = authorized_principals.into_iter().collect();
    });

    Ok(())
}
