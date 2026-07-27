use crate::guards::{caller_has_update_uploads_permission, GuardManagement};
use crate::state::mutate_state;
use crate::state::read_state;
pub use core_nft_api::__get_premint_entry_test;
pub use core_nft_api::__get_private_entry_test;
pub use core_nft_api::cancel_private_content_upload;
pub use core_nft_api::derive_vetkey_by_entry;
pub use core_nft_api::finalize_private_content_upload;
pub use core_nft_api::init_private_content_upload;
pub use core_nft_api::set_readers;
pub use core_nft_api::store_private_content_chunk;
pub use core_nft_api::{derive_vetkey, derive_vetkey_public_key};
use core_nft_common::resolve_readers;
use core_nft_common::types::management;
pub use core_nft_common::PrivateContentError;
use ic_cdk::update;
use serde_bytes::ByteBuf;

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn init_private_content_upload(
    args: init_private_content_upload::Args,
) -> init_private_content_upload::Response {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal =
        GuardManagement::new(caller).map_err(|e| PrivateContentError::StorageError(e))?;

    match args.token_id_opt.as_ref() {
        Some(token_id) => {
            let entry_name = args
                .entry_name
                .as_deref()
                .ok_or(core_nft_common::types::private_content::PrivateContentError::NotFound)?;

            let is_authorized = read_state(|state| {
                crate::guards::has_write_access(state, token_id, &entry_name, caller)
            });
            if !is_authorized {
                return Err(
                    core_nft_common::types::private_content::PrivateContentError::StorageError(
                        "Unauthorized".to_string(),
                    ),
                );
            }

            mutate_state(|state| {
                state.data.private_content_system.reencryption_validate(
                    token_id,
                    &entry_name,
                    &args.plaintext_hash,
                    args.plaintext_size,
                    args.file_size,
                    args.storage_canister_id,
                    args.storage_path.clone(),
                )
            })?;

            // Retrieve the canister_id and storage path from the existing private entry
            let (entry_canister_id, entry_storage_path) = read_state(|state| {
                let entry = state
                    .data
                    .private_content_system
                    .get_nft_private_entry(token_id.clone(), &entry_name)
                    .map_err(|_| {
                        PrivateContentError::StorageError("Private entry not found".to_string())
                    })?;
                Ok::<_, PrivateContentError>((
                    entry.storage_canister_id,
                    entry.storage_path.clone(),
                ))
            })?;

            // Check if the storage path is identical to the existing entry's storage path
            if args.storage_path != entry_storage_path {
                return Err(PrivateContentError::StorageError(
                    "Cannot change storage path during reupload".to_string(),
                ));
            }

            // Retrieve the canister directly
            let canister = read_state(|state| {
                state
                    .data
                    .sub_canister_manager
                    .get_canister(entry_canister_id)
            })
            .ok_or_else(|| PrivateContentError::StorageError("Canister not found".to_string()))?;

            let reupload_args = management::init_reupload::Args {
                file_path: args.storage_path.clone(),
                file_hash: hex::encode(args.file_hash),
                file_size: args.file_size,
                chunk_size: args.chunk_size,
            };

            canister
                .init_reupload(reupload_args)
                .await
                .map_err(|err| PrivateContentError::StorageError(format!("{:?}", err)))?;

            mutate_state(|state| {
                state.data.private_content_system.init_reencryption_store(
                    token_id,
                    &entry_name,
                    args.expected_chunks,
                    entry_canister_id,
                    args.storage_path,
                )
            })
        }
        None => {
            crate::guards::caller_has_update_uploads_permission()
                .map_err(|e| PrivateContentError::StorageError(e))?;

            mutate_state(|state| {
                state.data.private_content_system.init_upload_validate(
                    &args.plaintext_hash,
                    args.encryption_mode,
                    args.file_size,
                )
            })?;

            let mut sub_canister_manager =
                read_state(|state| state.data.sub_canister_manager.clone());
            let upload_args = management::init_upload::Args {
                file_path: args.storage_path.clone(),
                file_hash: hex::encode(args.file_hash),
                file_size: args.file_size,
                chunk_size: args.chunk_size,
            };

            let (_, canister_id) = sub_canister_manager
                .init_upload(upload_args)
                .await
                .map_err(|err| PrivateContentError::StorageError(err))?;

            mutate_state(|state| {
                state.data.sub_canister_manager = sub_canister_manager;
                let caller = ic_cdk::api::msg_caller();
                state.data.private_content_system.init_upload_register(
                    args.plaintext_hash,
                    args.salt,
                    caller,
                    args.readers,
                    canister_id,
                    args.storage_path,
                    args.encryption_mode,
                    args.plaintext_size,
                    args.expected_chunks,
                    args.file_size,
                    &args.default_readers,
                )
            })
        }
    }
}

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn store_private_content_chunk(
    args: store_private_content_chunk::Args,
) -> store_private_content_chunk::Response {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|_| {
        store_private_content_chunk::StorePrivateContentChunkError::ConcurrentManagementCall
    })?;

    if let Some(token_id) = args.token_id_opt.as_ref() {
        let entry_name = args
            .entry_name
            .as_deref()
            .ok_or(store_private_content_chunk::StorePrivateContentChunkError::NotFound)?;

        let is_authorized = read_state(|state| {
            crate::guards::has_write_access(state, token_id, &entry_name, caller)
        });
        if !is_authorized {
            return Err(
                store_private_content_chunk::StorePrivateContentChunkError::StorageCanisterError(
                    "Unauthorized".to_string(),
                ),
            );
        }
    } else {
        crate::guards::caller_has_update_uploads_permission().map_err(|e| {
            store_private_content_chunk::StorePrivateContentChunkError::StorageCanisterError(e)
        })?;
    }

    let chunk_index_nat = args.chunk_index.clone();
    let chunk_index = usize::try_from(chunk_index_nat.0)
        .map_err(|_| store_private_content_chunk::StorePrivateContentChunkError::InvalidChunk)?;
    let chunk_data = args.chunk_data.clone().into_vec();

    // Get the canister id
    let canister_id = read_state(|state| {
        if let Some(token_id) = args.token_id_opt.as_ref() {
            let entry_name = args
                .entry_name
                .as_deref()
                .ok_or(store_private_content_chunk::StorePrivateContentChunkError::NotFound)?;
            let entry = state
                .data
                .private_content_system
                .get_nft_private_entry(token_id.clone(), &entry_name)
                .map_err(|_| {
                    store_private_content_chunk::StorePrivateContentChunkError::NotFound
                })?;
            Ok::<_, store_private_content_chunk::StorePrivateContentChunkError>(
                entry.storage_canister_id,
            )
        } else {
            let entry = state
                .data
                .private_content_system
                .temp_file_cache
                .get(&args.plaintext_hash)
                .ok_or(store_private_content_chunk::StorePrivateContentChunkError::NotFound)?;
            Ok::<_, store_private_content_chunk::StorePrivateContentChunkError>(
                entry.storage_canister_id,
            )
        }
    })?;

    let canister = read_state(|state| state.data.sub_canister_manager.get_canister(canister_id))
        .ok_or_else(|| {
            store_private_content_chunk::StorePrivateContentChunkError::StorageCanisterError(
                "Canister not found".to_string(),
            )
        })?;

    let store_args = management::store_chunk::Args {
        chunk_id: args.chunk_index.clone(),
        file_path: args.storage_path.clone(),
        chunk_data: args.chunk_data.clone().into_vec(),
    };

    canister
        .store_chunk(store_args)
        .await
        .map_err(|err| store_private_content_chunk::StorePrivateContentChunkError::from(err))?;

    let result = mutate_state(|state| {
        if let Some(token_id) = args.token_id_opt.as_ref() {
            let entry_name = args
                .entry_name
                .as_deref()
                .ok_or(core_nft_common::types::private_content::PrivateContentError::NotFound)?;

            state.data.private_content_system.reupload_chunk_register(
                token_id,
                &entry_name,
                chunk_index,
                chunk_data,
            )
        } else {
            // entry_name removed from upload_chunk as per previous fix
            state.data.private_content_system.upload_chunk_register(
                &args.plaintext_hash,
                chunk_index,
                chunk_data,
            )
        }
    });

    match result {
        Ok(_) => Ok(()),
        Err(err) => Err(match err {
            core_nft_common::types::private_content::PrivateContentError::NotFound => {
                store_private_content_chunk::StorePrivateContentChunkError::NotFound
            }
            core_nft_common::types::private_content::PrivateContentError::InvalidChunk => {
                store_private_content_chunk::StorePrivateContentChunkError::InvalidChunk
            }
            core_nft_common::types::private_content::PrivateContentError::ContentTooLarge => {
                store_private_content_chunk::StorePrivateContentChunkError::ContentTooLarge
            }
            _ => store_private_content_chunk::StorePrivateContentChunkError::StorageCanisterError(
                format!("{:?}", err),
            ),
        }),
    }
}

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn finalize_private_content_upload(
    args: finalize_private_content_upload::Args,
) -> finalize_private_content_upload::Response {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|_| {
        finalize_private_content_upload::FinalizePrivateContentUploadError::ConcurrentManagementCall
    })?;

    if let Some(token_id) = args.token_id_opt.as_ref() {
        let entry_name = args
            .entry_name
            .as_deref()
            .ok_or(finalize_private_content_upload::FinalizePrivateContentUploadError::NotFound)?;

        let is_authorized = read_state(|state| {
            crate::guards::has_write_access(state, token_id, &entry_name, caller)
        });
        if !is_authorized {
            return Err(finalize_private_content_upload::FinalizePrivateContentUploadError::StorageCanisterError("Unauthorized".to_string()));
        }
    } else {
        crate::guards::caller_has_update_uploads_permission()
            .map_err(|e| finalize_private_content_upload::FinalizePrivateContentUploadError::StorageCanisterError(e))?;
    }

    let canister_id = read_state(|state| {
        if let Some(token_id) = args.token_id_opt.as_ref() {
            let entry_name = args.entry_name.as_deref().ok_or(
                finalize_private_content_upload::FinalizePrivateContentUploadError::NotFound,
            )?;
            let entry = state
                .data
                .private_content_system
                .get_nft_private_entry(token_id.clone(), &entry_name)
                .map_err(|_| {
                    finalize_private_content_upload::FinalizePrivateContentUploadError::NotFound
                })?;
            Ok::<_, finalize_private_content_upload::FinalizePrivateContentUploadError>(
                entry.storage_canister_id,
            )
        } else {
            let entry = state
                .data
                .private_content_system
                .temp_file_cache
                .get(&args.hash)
                .ok_or(
                    finalize_private_content_upload::FinalizePrivateContentUploadError::NotFound,
                )?;
            Ok::<_, finalize_private_content_upload::FinalizePrivateContentUploadError>(
                entry.storage_canister_id,
            )
        }
    })?;

    let canister = read_state(|state| {
        state.data.sub_canister_manager.get_canister(canister_id)
    }).ok_or_else(|| finalize_private_content_upload::FinalizePrivateContentUploadError::StorageCanisterError("Canister not found".to_string()))?;

    let finalize_args = management::finalize_upload::Args {
        file_path: args.storage_path.clone(),
    };

    canister
        .finalize_upload(finalize_args)
        .await
        .map_err(|err| {
            finalize_private_content_upload::FinalizePrivateContentUploadError::from(err)
        })?;

    // The router and `media_redirections` are keyed leading-slashed, because
    // `HttpRequest::get_path` always is. Registering the raw `storage_path`
    // would key a redirect that no request can ever match.
    let path = crate::utils::normalize_media_path(&args.storage_path);
    let redirection_url =
        read_state(|state| crate::utils::render_media_url_from_state(state, canister_id, &path));

    core_nft_common::types::http::add_redirection(path.clone(), redirection_url.clone());

    // The asset router is heap state and is wiped by every upgrade; post_upgrade
    // rebuilds it from `media_redirections` alone. Without this the redirect
    // would not survive the next collection upgrade.
    mutate_state(|state| {
        state
            .data
            .media_redirections
            .insert(path, redirection_url.clone());
    });

    let result = mutate_state(|state| {
        if let Some(token_id) = args.token_id_opt.as_ref() {
            let entry_name = args
                .entry_name
                .as_deref()
                .ok_or(core_nft_common::types::private_content::PrivateContentError::NotFound)?;

            state
                .data
                .private_content_system
                .finalize_reupload_register(token_id, &entry_name)
        } else {
            state
                .data
                .private_content_system
                .finalize_upload_register(&args.hash)
        }
    });

    match result {
        Ok(_) => Ok(()),
        Err(err) => Err(match err {
            core_nft_common::types::private_content::PrivateContentError::NotFound => {
                finalize_private_content_upload::FinalizePrivateContentUploadError::NotFound
            }
            core_nft_common::types::private_content::PrivateContentError::InvalidStateTransition => {
                finalize_private_content_upload::FinalizePrivateContentUploadError::InvalidStateTransition
            }
            core_nft_common::types::private_content::PrivateContentError::InvalidChunk => {
                finalize_private_content_upload::FinalizePrivateContentUploadError::IncompleteUpload
            }
            _ => finalize_private_content_upload::FinalizePrivateContentUploadError::StorageCanisterError(
                format!("{:?}", err),
            ),
        }),
    }
}

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn cancel_private_content_upload(
    args: cancel_private_content_upload::Args,
) -> cancel_private_content_upload::Response {
    let caller = ic_cdk::api::msg_caller();
    let _guard_principal = GuardManagement::new(caller).map_err(|_| {
        cancel_private_content_upload::CancelPrivateContentUploadError::ConcurrentManagementCall
    })?;

    let canister_id = read_state(|state| {
        let entry = state.data.private_content_system.temp_file_cache.get(&args.entry_hash)
            .ok_or(cancel_private_content_upload::CancelPrivateContentUploadError::UploadNotInitialized)?;
        Ok::<_, cancel_private_content_upload::CancelPrivateContentUploadError>(
            entry.storage_canister_id,
        )
    })?;

    let canister = read_state(|state| state.data.sub_canister_manager.get_canister(canister_id))
        .ok_or_else(|| {
            cancel_private_content_upload::CancelPrivateContentUploadError::StorageCanisterError(
                "Canister not found".to_string(),
            )
        })?;

    let cancel_args = management::cancel_upload::Args {
        file_path: args.storage_path.clone(),
    };

    canister
        .cancel_upload(cancel_args)
        .await
        .map_err(|err| cancel_private_content_upload::CancelPrivateContentUploadError::from(err))?;

    mutate_state(|state| {
        let _ = state
            .data
            .private_content_system
            .cancel_upload(&args.entry_hash);
    });

    Ok(())
}

#[update]
pub async fn derive_vetkey_public_key(
    _args: derive_vetkey_public_key::Args,
) -> derive_vetkey_public_key::Response {
    // FIXED: Use read_state instead of mutate_state
    let config = read_state(|state| state.data.private_content_system.config.clone());
    let pk = config.derive_public_key().await?;
    Ok(derive_vetkey_public_key::DeriveVetkeyPublicKeyResp {
        public_key: ByteBuf::from(pk),
    })
}

#[update]
pub async fn derive_vetkey(args: derive_vetkey::Args) -> derive_vetkey::Response {
    let caller = ic_cdk::api::msg_caller();
    let caller_text = caller.to_text();

    let identity_bytes = args.input.into_vec();

    let canonical_identity = String::from_utf8(identity_bytes.clone())
        .map_err(|_| "Invalid canonical identity encoding".to_string())?;

    let caller_in_identity = canonical_identity
        .split(',')
        .any(|principal| principal == caller_text);

    if !caller_in_identity {
        return Err("Caller is not part of the canonical identity".to_string());
    }

    let transport_public_key = args.transport_public_key.into_vec();

    let config = read_state(|state| state.data.private_content_system.config.clone());

    let encrypted_key = config
        .derive_vetkey(identity_bytes, transport_public_key)
        .await?;

    Ok(derive_vetkey::DeriveVetkeyResp {
        encrypted_key: ByteBuf::from(encrypted_key),
    })
}

#[update]
pub async fn derive_vetkey_by_entry(
    args: derive_vetkey_by_entry::Args,
) -> derive_vetkey_by_entry::Response {
    let caller = ic_cdk::api::msg_caller();

    let (canonical_identity, transport_key) = read_state(|state| {
        let token = state
            .data
            .get_token_by_id(&args.token_id)
            .ok_or_else(|| "NFT not found".to_string())?;

        let private_entry = state
            .data
            .private_content_system
            .get_nft_private_entry(args.token_id.clone(), &args.entry_name)?;

        // Check if the caller is the NFT owner
        let is_owner = token.token_owner.owner == caller;

        let record = state
            .data
            .private_content_system
            .nft_private
            .get(&args.token_id)
            .ok_or("Private content record not found")?;

        let effective_readers = resolve_readers(&private_entry.readers, &record.default_readers);
        let has_read_access = effective_readers.contains_key(&caller);

        // 1. Enforce access permissions
        if !is_owner && !has_read_access {
            return Err("The user does not have permission to derive a key".to_string());
        }

        // 2. Enforce state validation (PendingReencryption blocking non-owners)
        if private_entry.status == core_nft_common::PrivateContentStatus::PendingReencryption
            && !is_owner
        {
            return Err("Content is currently being re-encrypted by the owner".to_string());
        }

        // 3. Success path: Both owners and authorized readers reach here when valid
        Ok((
            private_entry.canonical_identity.clone(),
            args.transport_public_key.clone().into_vec(),
        ))
    })?;

    let config = read_state(|state| state.data.private_content_system.config.clone());

    let ek = config
        .derive_vetkey(canonical_identity, transport_key)
        .await?;

    Ok(derive_vetkey_by_entry::DeriveVetkeyResp {
        encrypted_key: ByteBuf::from(ek),
    })
}

#[update]
pub fn set_readers(args: set_readers::Args) -> set_readers::Response {
    let caller = ic_cdk::api::msg_caller();
    let is_authorized = read_state(|state| {
        crate::guards::has_write_access(state, &args.token_id, &args.entry_name, caller)
    });
    if !is_authorized {
        return Err("Caller is not authorized to set readers".to_string());
    }

    mutate_state(|state| {
        let nft_owner = {
            let token = state
                .data
                .get_token_by_id(&args.token_id)
                .ok_or_else(|| "NFT not found".to_string())?;

            token.token_owner.owner
        };

        state.data.private_content_system.set_readers(
            args.token_id,
            &args.entry_name,
            nft_owner,
            args.readers,
        )
    })
}

#[cfg(feature = "inttest")]
use ic_cdk_macros::query;
#[cfg(feature = "inttest")]
#[query]
pub fn __get_private_entry_test(
    args: __get_private_entry_test::Args,
) -> __get_private_entry_test::Response {
    let caller = ic_cdk::api::msg_caller();
    read_state(|state| {
        let token = state
            .data
            .get_token_by_id(&args.token_id)
            .ok_or_else(|| "NFT not found".to_string())?;

        let private_entry = state
            .data
            .private_content_system
            .get_nft_private_entry(args.token_id.clone(), &args.entry_name)?;

        let is_owner = token.token_owner.owner == caller;
        let record = state
            .data
            .private_content_system
            .nft_private
            .get(&args.token_id)
            .ok_or("Private content record not found")?;

        let effective_readers = resolve_readers(&private_entry.readers, &record.default_readers);
        let has_read_access = effective_readers.contains_key(&caller);

        let is_controller = ic_cdk::api::is_controller(&caller);

        if !is_owner && !has_read_access && !is_controller {
            return Err("Unauthorized".to_string());
        }

        Ok(private_entry)
    })
}

#[cfg(feature = "inttest")]
#[query]
pub fn __get_premint_entry_test(
    args: __get_premint_entry_test::Args,
) -> __get_premint_entry_test::Response {
    read_state(|state| {
        state
            .data
            .private_content_system
            .temp_file_cache
            .get(&args.hash)
            .cloned()
            .ok_or_else(|| "Not found".to_string())
    })
}
