use crate::guards::caller_has_update_uploads_permission;
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
pub use core_nft_common::types::management::{
    cancel_upload, finalize_upload, get_user_permissions, grant_permission, has_permission,
    init_upload, migration_icrc3_add_transaction, revoke_permission, store_chunk,
};
pub use core_nft_common::PrivateContentError;
use ic_cdk::update;
use serde_bytes::ByteBuf;

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn init_private_content_upload(
    args: init_private_content_upload::Args,
) -> init_private_content_upload::Response {
    match args.token_id_opt.as_ref() {
        Some(token_id) => {
            let entry_name = args
                .entry_name
                .as_deref()
                .ok_or(core_nft_common::types::private_content::PrivateContentError::NotFound)?;

            read_state(|state| {
                state.data.private_content_system.reencryption_validate(
                    token_id,
                    &entry_name,
                    &args.plaintext_hash,
                    args.plaintext_size,
                    args.file_size,
                )
            })?;

            let args_cloned = args.clone();
            crate::updates::management::init_upload(args_cloned.into())
                .await
                .map(|_| ())
                .map_err(|err| PrivateContentError::StorageError(format!("{:?}", err)))?;

            mutate_state(|state| {
                state.data.private_content_system.init_reencryption_store(
                    token_id,
                    &entry_name,
                    args.expected_chunks,
                )
            })
        }
        None => {
            read_state(|state| {
                state.data.private_content_system.init_premint_validate(
                    &args.plaintext_hash,
                    args.encryption_mode,
                    args.file_size,
                )
            })?;

            let args_cloned = args.clone();
            crate::updates::management::init_upload(args_cloned.into())
                .await
                .map(|_| ())
                .map_err(|err| PrivateContentError::StorageError(format!("{:?}", err)))?;

            mutate_state(|state| {
                let caller = ic_cdk::api::msg_caller();
                state.data.private_content_system.init_premint_store(
                    args.plaintext_hash,
                    args.salt,
                    caller,
                    args.readers,
                    args.storage_canister_id,
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
    let chunk_index_nat = args.chunk_index.clone();
    let chunk_index = usize::try_from(chunk_index_nat.0)
        .map_err(|_| store_private_content_chunk::StorePrivateContentChunkError::InvalidChunk)?;
    let chunk_data = args.chunk_data.clone().into_vec();

    let storage_result = crate::updates::management::store_chunk(args.clone().into())
        .await
        .map(|_| ())
        .map_err(Into::into);

    if let Err(err) = storage_result {
        return Err(err);
    }

    let result = mutate_state(|state| {
        if let Some(token_id) = args.token_id_opt.as_ref() {
            let entry_name = args
                .entry_name
                .as_deref()
                .ok_or(core_nft_common::types::private_content::PrivateContentError::NotFound)?;

            state.data.private_content_system.reupload_chunk(
                token_id,
                &entry_name,
                chunk_index,
                chunk_data,
            )
        } else {
            // entry_name removed from upload_chunk as per previous fix
            state.data.private_content_system.upload_chunk(
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
    let storage_result = crate::updates::management::finalize_upload(args.clone().into())
        .await
        .map(|_| ())
        .map_err(Into::into);

    if let Err(err) = storage_result {
        return Err(err);
    }

    let result = mutate_state(|state| {
        if let Some(token_id) = args.token_id_opt.as_ref() {
            let entry_name = args
                .entry_name
                .as_deref()
                .ok_or(core_nft_common::types::private_content::PrivateContentError::NotFound)?;

            state
                .data
                .private_content_system
                .finalize_reupload(token_id, &entry_name)
        } else {
            state
                .data
                .private_content_system
                .finalize_upload(&args.hash)
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
    let storage_result = crate::updates::management::cancel_upload(args.clone().into())
        .await
        .map(|_| ())
        .map_err(Into::into);

    if let Err(err) = storage_result {
        return Err(err);
    }

    mutate_state(|state| {
        let _ = state
            .data
            .private_content_system
            .cancel_upload(&args.entry_hash);
    });

    Ok(())
}

#[update(guard = "caller_has_update_uploads_permission")]
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

#[update(guard = "caller_has_update_uploads_permission")]
pub fn set_readers(args: set_readers::Args) -> set_readers::Response {
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
#[query]
pub fn __get_private_entry_test(
    args: __get_private_entry_test::Args,
) -> __get_private_entry_test::Response {
    read_state(|state| {
        state
            .data
            .private_content_system
            .get_nft_private_entry(args.token_id, &args.entry_name)
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
            .premint_cache
            .get(&args.hash)
            .cloned()
            .ok_or_else(|| "Not found".to_string())
    })
}
