use crate::guards::{ caller_is_governance_principal, GuardManagement };
use crate::state::{ mutate_state, read_state, InternalFilestorageData };
use crate::types::sub_canister::StorageCanister;
use crate::types::{ icrc7, management, nft };
use crate::utils::{ check_memo, hash_string_to_u64, trace };
use candid::types::principal;
use candid::Nat;
use ic_cdk::api::call::RejectionCode;
use ic_cdk_macros::update;
pub use storage_api_canister::cancel_upload;
pub use storage_api_canister::delete_file;
use crate::types::http::{ add_redirection, remove_redirection };
pub use storage_api_canister::init_upload;
pub use storage_api_canister::store_chunk;
pub use storage_api_canister::finalize_upload;

//TODO Use minting autority to mint tokens
#[update(guard = "caller_is_governance_principal")]
pub fn mint(req: management::mint::Args) -> management::mint::Response {
    let caller = ic_cdk::caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|e| (
        RejectionCode::CanisterError,
        e,
    ))?;

    let token_name_hash = Nat::from(hash_string_to_u64(&req.token_name));

    let token_list = read_state(|state| state.data.tokens_list.clone());
    let supply_cap = read_state(|state| {
        state.data.supply_cap.clone().unwrap_or(Nat::from(icrc7::DEFAULT_MAX_SUPPLY_CAP))
    });

    if token_list.len() > supply_cap {
        return Err((RejectionCode::CanisterError, "Exceed Max allowed Supply Cap".to_string()));
    }

    match check_memo(req.memo) {
        Ok(_) => {}
        Err(e) => {
            return Err((RejectionCode::CanisterError, e));
        }
    }

    match token_list.contains_key(&token_name_hash.clone()) {
        true => {
            return Err((RejectionCode::CanisterError, "Token already exists".to_string()));
        }
        false => {
            let new_token = nft::Icrc7Token::new(
                token_name_hash.clone(),
                req.token_name,
                req.token_description,
                req.token_logo,
                req.token_owner
            );
            mutate_state(|state| {
                state.data.tokens_list.insert(token_name_hash.clone(), new_token);
            });

            // TODO add transactions logs
        }
    }

    Ok(token_name_hash.clone())
}

#[update(guard = "caller_is_governance_principal")]
pub async fn update_nft_metadata(
    req: management::update_nft_metadata::Args
) -> management::update_nft_metadata::Response {
    trace("Updating NFT metadata");
    let caller = ic_cdk::caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|e| (
        RejectionCode::CanisterError,
        e,
    ))?;

    let token_name_hash = req.token_id;

    let token_list = read_state(|state| state.data.tokens_list.clone());

    match token_list.contains_key(&token_name_hash.clone()) {
        true => {
            let mut token = token_list.get(&token_name_hash.clone()).unwrap().clone();
            if let Some(name) = req.token_name {
                token.token_name = name;
            }
            if let Some(description) = req.token_description {
                token.token_description = Some(description);
            }
            if let Some(logo) = req.token_logo {
                token.token_logo = Some(logo);
            }
            if let Some(metadata) = req.token_metadata {
                trace(&format!("Adding metadata to token: {:?}", metadata));
                token.add_metadata(metadata).await;
            }
            mutate_state(|state| {
                state.data.tokens_list.insert(token_name_hash.clone(), token);
            });
            trace(&format!("Updated NFT metadata for token: {:?}", token_name_hash.clone()));
        }
        false => {
            return Err((RejectionCode::CanisterError, "Token does not exist".to_string()));
        }
    }

    // TODO add transactions logs

    Ok(token_name_hash.clone())
}

// #[update(guard = "caller_is_governance_principal")]
// pub fn burn_nft() -> () {
//     token.burn()
// }

#[update(guard = "caller_is_governance_principal")]
pub fn update_minting_authorities(
    req: management::update_minting_authorities::Args
) -> management::update_minting_authorities::Response {
    let caller = ic_cdk::caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|e| (
        RejectionCode::CanisterError,
        e,
    ))?;

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
    req: management::remove_minting_authorities::Args
) -> management::remove_minting_authorities::Response {
    let caller = ic_cdk::caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|e| (
        RejectionCode::CanisterError,
        e,
    ))?;

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
    trace(&format!("Initiate file upload: {:?}", data));
    let caller = ic_cdk::caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|e| (
        RejectionCode::CanisterError,
        e,
    ))?;

    if read_state(|state| state.internal_filestorage.contains_path(&data.file_path)) {
        return Err((RejectionCode::CanisterError, "File exists.".to_string()));
    }

    let mut sub_canister_manager = read_state(|state| state.data.sub_canister_manager.clone());

    let canister = match sub_canister_manager.init_upload(data.clone()).await {
        Ok((_, canister)) => {
            trace(&format!("Initiated file upload: {:?}", data));
            canister
        }
        Err(e) => {
            trace(&format!("Error inserting data: {:?}", e));
            return Err((RejectionCode::CanisterError, e));
        }
    };

    mutate_state(|state| {
        state.data.sub_canister_manager = sub_canister_manager;
        state.internal_filestorage.insert(data.file_path.clone(), InternalFilestorageData {
            init_timestamp: ic_cdk::api::time(),
            state: crate::state::UploadState::Init,
            canister: canister,
            path: data.file_path,
        });
    });

    Ok(init_upload::InitUploadResp {})
}

#[update(guard = "caller_is_governance_principal")]
pub async fn store_chunk(data: store_chunk::Args) -> store_chunk::Response {
    let caller = ic_cdk::caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|e| (
        RejectionCode::CanisterError,
        e,
    ))?;

    let (init_timestamp, canister_id, file_path) = match
        read_state(|state| { state.internal_filestorage.get(&data.file_path).cloned() })
    {
        Some(data) => {
            match data.state {
                crate::state::UploadState::Init => {
                    (data.init_timestamp, data.canister, data.path)
                }
                crate::state::UploadState::InProgress => {
                    (data.init_timestamp, data.canister, data.path)
                }
                crate::state::UploadState::Finalized => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Upload already finalized".to_string(),
                    ));
                }
            }
        }
        None => {
            return Err((RejectionCode::CanisterError, "Upload not initiated".to_string()));
        }
    };

    let canister: StorageCanister = match
        read_state(|state| state.data.sub_canister_manager.get_canister(canister_id.clone()))
    {
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
        Ok(_) => {
            trace(&format!("Stored chunk: {:?}", data));
        }
        Err(e) => {
            trace(&format!("Error storing chunk: {:?}", e));
            return Err((RejectionCode::CanisterError, e));
        }
    }
    mutate_state(|state| {
        state.internal_filestorage.insert(data.file_path.clone(), InternalFilestorageData {
            init_timestamp: init_timestamp,
            state: crate::state::UploadState::InProgress,
            canister: canister_id,
            path: file_path,
        });
    });

    Ok(store_chunk::StoreChunkResp {})
}

#[update(guard = "caller_is_governance_principal")]
pub async fn finalize_upload(data: finalize_upload::Args) -> finalize_upload::Response {
    let caller = ic_cdk::caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|e| (
        RejectionCode::CanisterError,
        e,
    ))?;

    let (init_timestamp, media_path, canister_id) = match
        read_state(|state| { state.internal_filestorage.get(&data.file_path).cloned() })
    {
        Some(data) => {
            match data.state {
                crate::state::UploadState::Init => {
                    return Err((RejectionCode::CanisterError, "Upload didnt started".to_string()));
                }
                crate::state::UploadState::InProgress => {
                    (data.init_timestamp, data.path, data.canister)
                }
                crate::state::UploadState::Finalized => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Upload already finalized".to_string(),
                    ));
                }
            }
        }
        None => {
            return Err((RejectionCode::CanisterError, "Upload not initiated".to_string()));
        }
    };

    let canister: StorageCanister = match
        read_state(|state| state.data.sub_canister_manager.get_canister(canister_id.clone()))
    {
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
        Ok(_) => {
            trace(&format!("Stored chunk: {:?}", data));
        }
        Err(e) => {
            trace(&format!("Error storing chunk: {:?}", e));
            // TODO shall we automaticly cleanup or add a cleanup and let user retry?
            return Err((RejectionCode::CanisterError, e));
        }
    }

    let redirection_url = format!("https://{}.raw.icp0.io{}", canister_id, media_path.clone());
    add_redirection(media_path.clone(), redirection_url);

    mutate_state(|state| {
        state.internal_filestorage.insert(data.file_path.clone(), InternalFilestorageData {
            init_timestamp: init_timestamp,
            state: crate::state::UploadState::Finalized,
            canister: canister_id,
            path: media_path,
        });
    });

    return Ok(finalize_upload::FinalizeUploadResp {});
}

#[update(guard = "caller_is_governance_principal")]
pub async fn cancel_upload(data: cancel_upload::Args) -> cancel_upload::Response {
    let caller = ic_cdk::caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|e| (
        RejectionCode::CanisterError,
        e,
    ))?;

    let (media_path, canister_id) = match
        read_state(|state| { state.internal_filestorage.get(&data.file_path).cloned() })
    {
        Some(data) => {
            match data.state {
                crate::state::UploadState::Init => { (data.path, data.canister) }
                crate::state::UploadState::InProgress => { (data.path, data.canister) }
                crate::state::UploadState::Finalized => {
                    return Err((
                        RejectionCode::CanisterError,
                        "Upload already finalized".to_string(),
                    ));
                }
            }
        }
        None => {
            return Err((RejectionCode::CanisterError, "Upload not initiated".to_string()));
        }
    };

    let canister: StorageCanister = match
        read_state(|state| state.data.sub_canister_manager.get_canister(canister_id.clone()))
    {
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
        Ok(_) => {
            trace(&format!("Stored chunk: {:?}", data));
        }
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
    let _guard_principal = GuardManagement::new(caller).map_err(|e| (
        RejectionCode::CanisterError,
        e,
    ))?;

    let (media_path, canister_id) = match
        read_state(|state| { state.internal_filestorage.get(&data.file_path).cloned() })
    {
        Some(data) => {
            match data.state {
                crate::state::UploadState::Init => {
                    return Err((RejectionCode::CanisterError, "Upload didnt started".to_string()));
                }
                crate::state::UploadState::InProgress => {
                    return Err((RejectionCode::CanisterError, "Upload in progress".to_string()));
                }
                crate::state::UploadState::Finalized => { (data.path, data.canister) }
            }
        }
        None => {
            return Err((RejectionCode::CanisterError, "Upload not initiated".to_string()));
        }
    };

    let canister: StorageCanister = match
        read_state(|state| state.data.sub_canister_manager.get_canister(canister_id.clone()))
    {
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
        Ok(_) => {
            trace(&format!("Stored chunk: {:?}", data));
        }
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
        format!("https://{}.raw.icp0.io{}", canister_id, media_path.clone())
    );

    Ok(delete_file::DeleteFileResp {})
}
