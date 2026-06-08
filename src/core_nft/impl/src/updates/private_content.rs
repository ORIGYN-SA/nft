use crate::guards::caller_has_update_uploads_permission;
use crate::state::mutate_state;
pub use core_nft_api::cancel_private_content_upload;
pub use core_nft_api::finalize_private_content_upload;
pub use core_nft_api::init_private_content_upload;
pub use core_nft_api::store_private_content_chunk;
pub use core_nft_api::{derive_vetkey, derive_vetkey_public_key};
pub use core_nft_common::types::management::{
    cancel_upload, finalize_upload, get_user_permissions, grant_permission, has_permission,
    init_upload, migration_icrc3_add_transaction, revoke_permission, store_chunk,
};
use ic_cdk::update;
use serde_bytes::ByteBuf;

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn init_private_upload(
    args: init_private_content_upload::Args,
) -> init_private_content_upload::Response {
    // Step 1: Validate parameters (AES block alignment, size checks, cache conflicts)
    let validate_result = mutate_state(|state| {
        state.data.private_content_system.init_premint_validate(
            &args.hash,
            args.encryption_mode.clone(),
            args.plaintext_size,
            args.total_size,
        )
    });

    if let Err(err) = validate_result {
        return Err(match err {
            core_nft_common::types::private_content::PrivateContentError::AlreadyExists => {
                init_private_content_upload::InitPrivateContentUploadError::AlreadyExists
            }
            core_nft_common::types::private_content::PrivateContentError::ContentTooLarge => {
                init_private_content_upload::InitPrivateContentUploadError::ContentTooLarge
            }
            core_nft_common::types::private_content::PrivateContentError::StorageError(msg) => {
                init_private_content_upload::InitPrivateContentUploadError::StorageCanisterError(
                    msg,
                )
            }
            _ => init_private_content_upload::InitPrivateContentUploadError::StorageCanisterError(
                format!("{:?}", err),
            ),
        });
    }

    // Step 2: Initialize storage upload on remote canister
    let args_cloned = args.clone();
    let storage_result = crate::updates::management::init_upload(args_cloned.clone().into()).await;

    if let Err(err) = storage_result {
        return Err(match err {
            core_nft_common::types::management::init_upload::InitUploadError::FileAlreadyExists => {
                init_private_content_upload::InitPrivateContentUploadError::AlreadyExists
            }
            core_nft_common::types::management::init_upload::InitUploadError::StorageCanisterError(msg) => {
                init_private_content_upload::InitPrivateContentUploadError::StorageCanisterError(msg)
            }
            _ => init_private_content_upload::InitPrivateContentUploadError::StorageCanisterError(
                format!("{:?}", err),
            ),
        });
    }

    // Step 3: Store the entry in local premint cache
    let store_result = mutate_state(|state| {
        state.data.private_content_system.init_premint_store(
            args.hash,
            args.default_readers,
            args.entry_name,
            args.storage_canister_id,
            args.storage_path,
            args.encryption_mode.clone(),
            args.plaintext_size,
            args.expected_chunks,
            args.total_size,
        )
    });

    crate::updates::management::init_upload(args_cloned.into())
        .await
        .map(|_| ())
        .map_err(Into::into)
}

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn store_private_chunk(
    args: store_private_content_chunk::Args,
) -> store_private_content_chunk::Response {
    let hash = args.hash;
    let entry_name = args.entry_name.clone();
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
        state
            .data
            .private_content_system
            .upload_chunk(&hash, &entry_name, chunk_index, chunk_data)
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
pub async fn finalize_private_upload(
    args: finalize_private_content_upload::Args,
) -> finalize_private_content_upload::Response {
    // Step 1: Finalize upload on remote storage canister
    let storage_result = crate::updates::management::finalize_upload(args.clone().into())
        .await
        .map(|_| ())
        .map_err(Into::into);

    if let Err(err) = storage_result {
        return Err(err);
    }

    // Step 2: Transition status from PendingUpload to PendingMinting in local cache
    let result = mutate_state(|state| {
        state
            .data
            .private_content_system
            .finalize_upload(&args.hash, &args.entry_name)
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
pub async fn cancel_private_upload(
    args: cancel_private_content_upload::Args,
) -> cancel_private_content_upload::Response {
    // Step 1: Cancel upload on remote storage canister
    let storage_result = crate::updates::management::cancel_upload(args.clone().into())
        .await
        .map(|_| ())
        .map_err(Into::into);

    if let Err(err) = storage_result {
        return Err(err);
    }

    // Step 2: Remove from local premint cache
    mutate_state(|state| {
        let _ = state
            .data
            .private_content_system
            .cancel_upload(&args.entry_name);
    });

    Ok(())
}

/// Retrieves the vetKD verification key for this canister.
/// This key is used to verify the authenticity of derived vetKeys.
#[update(guard = "caller_has_update_uploads_permission")]
pub async fn derive_vetkey_public_key(
    args: derive_vetkey_public_key::Args,
) -> derive_vetkey_public_key::Response {
    let config = mutate_state(|state| state.data.private_content_system.config.clone());
    let pk = config.derive_public_key(args.context.into_vec()).await;
    Ok(derive_vetkey_public_key::DeriveVetkeyPublicKeyResp {
        public_key: ByteBuf::from(pk),
    })
}

#[update(guard = "caller_has_update_uploads_permission")]
pub async fn derive_vetkey(args: derive_vetkey::Args) -> derive_vetkey::Response {
    let config = mutate_state(|state| state.data.private_content_system.config.clone());
    let ek = config
        .derive_vetkey(
            args.context.into_vec(),
            args.input.into_vec(),
            args.transport_public_key.into_vec(),
        )
        .await;

    Ok(derive_vetkey::DeriveVetkeyResp {
        encrypted_key: ByteBuf::from(ek),
    })
}
