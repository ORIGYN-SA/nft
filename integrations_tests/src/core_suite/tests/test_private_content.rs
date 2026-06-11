use crate::client::core_nft::*;
use crate::client::core_nft::{
    cancel_upload, finalize_upload, get_upload_status, grant_permission, init_upload, mint,
    revoke_permission, store_chunk, update_collection_metadata, update_nft_metadata,
};
use crate::core_suite::setup::default_test_setup;
use crate::core_suite::setup::setup::TestEnv;
use crate::utils::create_default_icrc97_metadata;
use crate::utils::{
    extract_metadata_file_path, fetch_metadata_json, setup_http_client, upload_file,
    upload_metadata,
};
use aes_gcm::aead::Aead;
use aes_gcm::Aes256Gcm;
use aes_gcm::KeyInit;
use bity_ic_storage_canister_api::types::storage::UploadState;
use bytes::Bytes;
use candid::{Encode, Nat, Principal};
use core_nft_api::cancel_private_content_upload;
use core_nft_api::derive_vetkey;
use core_nft_api::derive_vetkey_public_key;
use core_nft_api::init_private_content_upload;
use core_nft_common::types::management::{
    cancel_upload, finalize_upload, grant_permission, init_upload, mint, mint::MintRequest,
    revoke_permission, store_chunk, update_collection_metadata, update_nft_metadata,
};
use core_nft_common::types::permissions::Permission;
use core_nft_common::AccessRights;
use core_nft_common::EncryptionMode;
use core_nft_common::PrivateContentStatus;
use core_nft_common::PrivateEntry;
use core_nft_common::{construct_canonical_identity, ReaderInfo};
use http::Request;
use http_body_util::BodyExt;
use ic_agent::Agent;
use ic_cdk::println;
use ic_http_gateway::{HttpGatewayClient, HttpGatewayRequestArgs};
use ic_vetkeys::DerivedPublicKey;
use ic_vetkeys::EncryptedVetKey;
use ic_vetkeys::TransportSecretKey;
use icrc_ledger_types::icrc1::account::Account;
use serde_bytes::ByteBuf;
use serde_json::{self, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[test]
fn test_private_content_upload_and_mint_rand_bytes() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;

    let grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    );

    let content = b"0123456789abcdef".to_vec();
    let plaintext_size = content.len() as u64;
    let file_size = plaintext_size;
    let entry_name = "/private_test.bin".to_string();
    let storage_path = "/private/test.bin".to_string();

    let hash_bytes = Sha256::digest(&content);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);
    let salt = vec![];

    let init_args = core_nft_api::init_private_content_upload::Args {
        token_id_opt: None,
        plaintext_hash: hash,
        file_hash: hash,
        salt: salt.clone(),
        entry_name: entry_name.clone(),
        default_readers: HashMap::new(),
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        plaintext_size,
        expected_chunks: 1,
        chunk_size: Some(plaintext_size),
        file_size,
        encryption_mode: EncryptionMode::AES256,
    };

    let init_response =
        init_private_content_upload(pic, controller, collection_canister_id, &init_args);
    assert!(
        init_response.is_ok(),
        "init_private_content_upload failed: {:?}",
        init_response
    );

    let store_response = store_private_content_chunk(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            plaintext_hash: hash,
            entry_name: entry_name.clone(),
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(content.clone()),
        },
    );
    assert!(
        store_response.is_ok(),
        "store_private_content_chunk failed: {:?}",
        store_response
    );

    let finalize_response = finalize_private_content_upload(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            hash,
            entry_name: entry_name.clone(),
        },
    );
    assert!(
        finalize_response.is_ok(),
        "finalize_private_content_upload failed: {:?}",
        finalize_response
    );

    let private_entry = PrivateEntry {
        status: PrivateContentStatus::PendingMinting,
        readers: HashMap::new(),
        hash,
        salt,
        plaintext_size,
        file_size,
        encryption_mode: EncryptionMode::AES256,
        canonical_identity: construct_canonical_identity(&HashMap::new()),
        previous_canonical_identity: None,
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        pending_upload: None,
        format_version: 1,
    };

    let mint_response = mint(
        pic,
        controller,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(private_entry),
            }],
        },
    );
    assert!(
        mint_response.is_ok(),
        "mint with private content failed: {:?}",
        mint_response
    );
}

#[test]
fn test_private_content_encryption_and_decryption_with_vetkeys() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;

    let _grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    );
    let _grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateUploads,
        }),
    );

    let default_readers = HashMap::new();
    let plaintext = b"Confidential NFT Asset Data 2026".to_vec();
    let plaintext_size = plaintext.len() as u64;
    let entry_name = "/test.txt".to_string();
    let storage_path = "/private/test.txt".to_string();
    let mut readers = HashMap::new();
    readers.insert(
        nft_owner1,
        ReaderInfo {
            rights: AccessRights::ReadWriteManage,
            alias: None,
        },
    );
    let canonical_identity = construct_canonical_identity(&readers);

    // 1. Calculate metadata for the plaintext BEFORE encryption
    let plaintext_hash_bytes = Sha256::digest(&plaintext);
    let mut plaintext_hash = [0u8; 32];
    plaintext_hash.copy_from_slice(&plaintext_hash_bytes);

    // NOTE: Salt tied to the specific file upload entry, but better to make random
    let salt = Sha256::digest(entry_name.as_bytes()).to_vec();

    // -------------------------------------------------------------------------
    // STEP 0: Generate transport key
    // -------------------------------------------------------------------------
    let transport_seed = [102u8; 32]; // Differentiated from crypto_seed for realism
    let tsk = TransportSecretKey::from_seed(transport_seed.to_vec())
        .expect("Failed to initialize TransportSecretKey from seed");

    let tpk = tsk.public_key();

    // -------------------------------------------------------------------------
    // STEP 1: Fetch the Master VetKey Public Key from the Canister
    // -------------------------------------------------------------------------
    let pub_key_response = derive_vetkey_public_key(pic, controller, collection_canister_id, &())
        .expect("Canister returned an error deriving public key");

    // Reconstruct the DerivedPublicKey object from the canister response bytes
    let dpk = DerivedPublicKey::deserialize(&pub_key_response.public_key)
        .expect("Failed to deserialize public key using ic-vetkeys");

    // -------------------------------------------------------------------------
    // STEP 2: Derive VetKey
    // -------------------------------------------------------------------------
    // The client requests the decryption key by providing the exact derived identity definition
    let derive_args = derive_vetkey::Args {
        input: ByteBuf::from(canonical_identity.clone()), // Requesting key specifically for this salted context
        transport_public_key: ByteBuf::from(tpk),
    };

    let derive_response = derive_vetkey(pic, nft_owner1, collection_canister_id, &derive_args)
        .expect("Canister or Management system rejected VetKey derivation");

    // -------------------------------------------------------------------------
    // STEP 3: Derive symmetrical key
    // -------------------------------------------------------------------------
    let encrypted_vetkey = EncryptedVetKey::deserialize(&derive_response.encrypted_key)
        .expect("Failed to parse returned payload into EncryptedVetKey");

    // Decrypt and verify using the exact salted identity parameters
    let vetkey = encrypted_vetkey
        .decrypt_and_verify(&tsk, &dpk, &canonical_identity)
        .expect("Identity-Based verification and decryption of VetKey failed");

    let sk = vetkey.derive_symmetric_key("", 32);
    // -------------------------------------------------------------------------
    // STEP 4: Encrypt file
    // -------------------------------------------------------------------------
    let cipher = Aes256Gcm::new_from_slice(&sk)
        .expect("Failed to initialize AES-256-GCM cipher with derived key");

    let deterministic_nonce_bytes = Sha256::digest(entry_name.as_bytes());
    let nonce = aes_gcm::Nonce::from_slice(&deterministic_nonce_bytes[0..12]); // First 12 bytes

    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).unwrap();
    let file_size = ciphertext.len() as u64;
    println!("ciphertext length: {:?}", ciphertext.len());
    // -------------------------------------------------------------------------
    // STEP 4: Upload encrypted file
    // -------------------------------------------------------------------------
    // Keeping hash, plaintext_size, and file_size tracked to plaintext as requested.
    let hash_bytes = Sha256::digest(&ciphertext);
    let mut file_hash = [0u8; 32];
    file_hash.copy_from_slice(&hash_bytes);

    let init_args = core_nft_api::init_private_content_upload::Args {
        token_id_opt: None,
        plaintext_hash: plaintext_hash,
        file_hash: file_hash,
        salt: salt.clone(),
        entry_name: entry_name.clone(),
        default_readers: default_readers.clone(),
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        plaintext_size,
        expected_chunks: 1,
        chunk_size: Some(file_size), // Chunk size handles payload container dimensions
        file_size,
        encryption_mode: EncryptionMode::AES256,
    };

    let init_response =
        init_private_content_upload(pic, controller, collection_canister_id, &init_args);
    assert!(init_response.is_ok(), "init_private_content_upload failed");

    let store_response = store_private_content_chunk(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            plaintext_hash: plaintext_hash, // NOTE: encrypted chunk hash
            entry_name: entry_name.clone(),
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext.clone()), // Uploads the actual ciphertext
        },
    );
    println!("store_response: {:?}", store_response);
    assert!(store_response.is_ok(), "store_private_content_chunk failed");

    let finalize_response = finalize_private_content_upload(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            hash: plaintext_hash,
            entry_name: entry_name.clone(),
        },
    );
    println!("finalize_response: {:?}", finalize_response);
    assert!(
        finalize_response.is_ok(),
        "finalize_private_content_upload failed"
    );

    let private_entry = PrivateEntry {
        status: PrivateContentStatus::PendingMinting,
        readers: HashMap::new(),
        hash: plaintext_hash,
        salt: salt.clone(),
        plaintext_size,
        file_size,
        encryption_mode: EncryptionMode::AES256,
        canonical_identity: canonical_identity.to_vec().clone(),
        previous_canonical_identity: None,
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        pending_upload: None,
        format_version: 1,
    };

    let mint_response = mint(
        pic,
        controller,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(private_entry),
            }],
        },
    );
    assert!(mint_response.is_ok(), "Mint with private content failed");

    // -------------------------------------------------------------------------
    // STEP 4: Generate transport key
    // -------------------------------------------------------------------------
    let transport_seed = [101u8; 32]; // Differentiated from the first one
    let tsk = TransportSecretKey::from_seed(transport_seed.to_vec())
        .expect("Failed to initialize TransportSecretKey from seed");

    let tpk = tsk.public_key();

    // -------------------------------------------------------------------------
    // STEP 5: Request Decryption VetKey as the Authorized NFT Owner
    // -------------------------------------------------------------------------
    // The client requests the decryption key by providing the exact derived identity definition
    let derive_args = derive_vetkey::Args {
        input: ByteBuf::from(canonical_identity.clone()), // Requesting key specifically for this salted context
        transport_public_key: ByteBuf::from(tpk),
    };

    let derive_response = derive_vetkey(pic, nft_owner1, collection_canister_id, &derive_args)
        .expect("Canister or Management system rejected VetKey derivation");

    // -------------------------------------------------------------------------
    // STEP 6: Decrypt the Encrypted VetKey wrapper & Parse Content Plaintext
    // -------------------------------------------------------------------------
    let encrypted_vetkey = EncryptedVetKey::deserialize(&derive_response.encrypted_key)
        .expect("Failed to parse returned payload into EncryptedVetKey");

    // Decrypt and verify using the exact salted identity parameters
    let vetkey = encrypted_vetkey
        .decrypt_and_verify(&tsk, &dpk, &canonical_identity)
        .expect("Identity-Based verification and decryption of VetKey failed");

    let sk1 = vetkey.derive_symmetric_key("", 32);
    let cipher1 = Aes256Gcm::new_from_slice(&sk1)
        .expect("Failed to initialize AES-256-GCM cipher with derived key");

    // Reuse of the initial plaintext (simulated download from storage canister)
    let decrypted_plaintext = cipher1
        .decrypt(nonce, ciphertext.as_ref())
        .expect("Error while decryption");
    // println!("decrypted_plaintext: {:?}", decrypted_plaintext);

    // Final verification assertion
    assert_eq!(
        decrypted_plaintext, plaintext,
        "Decrypted data mismatch: Core asset content was corrupted or incorrectly decoded!"
    );
}

#[test]
fn test_private_content_encryption_unauthorized_access() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;

    let _grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    );
    let _grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateUploads,
        }),
    );

    let default_readers = HashMap::new();
    let plaintext = b"Confidential NFT Asset Data 2026".to_vec();
    let plaintext_size = plaintext.len() as u64;
    let entry_name = "/test.txt".to_string();
    let storage_path = "/private/test.txt".to_string();
    let mut readers = HashMap::new();
    readers.insert(
        nft_owner1,
        ReaderInfo {
            rights: AccessRights::ReadWriteManage,
            alias: None,
        },
    );
    let canonical_identity = construct_canonical_identity(&readers);

    // 1. Calculate metadata for the plaintext BEFORE encryption
    let plaintext_hash_bytes = Sha256::digest(&plaintext);
    let mut plaintext_hash = [0u8; 32];
    plaintext_hash.copy_from_slice(&plaintext_hash_bytes);

    // NOTE: Salt tied to the specific file upload entry, but better to make random
    let salt = Sha256::digest(entry_name.as_bytes()).to_vec();

    // -------------------------------------------------------------------------
    // STEP 0: Generate transport key
    // -------------------------------------------------------------------------
    let transport_seed = [102u8; 32]; // Differentiated from crypto_seed for realism
    let tsk = TransportSecretKey::from_seed(transport_seed.to_vec())
        .expect("Failed to initialize TransportSecretKey from seed");

    let tpk = tsk.public_key();

    // -------------------------------------------------------------------------
    // STEP 1: Fetch the Master VetKey Public Key from the Canister
    // -------------------------------------------------------------------------
    let pub_key_response = derive_vetkey_public_key(pic, controller, collection_canister_id, &())
        .expect("Canister returned an error deriving public key");

    // Reconstruct the DerivedPublicKey object from the canister response bytes
    let dpk = DerivedPublicKey::deserialize(&pub_key_response.public_key)
        .expect("Failed to deserialize public key using ic-vetkeys");

    // -------------------------------------------------------------------------
    // STEP 2: Derive VetKey
    // -------------------------------------------------------------------------
    // The client requests the decryption key by providing the exact derived identity definition
    let derive_args = derive_vetkey::Args {
        input: ByteBuf::from(canonical_identity.clone()), // Requesting key specifically for this salted context
        transport_public_key: ByteBuf::from(tpk),
    };

    let derive_response = derive_vetkey(pic, nft_owner1, collection_canister_id, &derive_args)
        .expect("Canister or Management system rejected VetKey derivation");

    // -------------------------------------------------------------------------
    // STEP 3: Derive symmetrical key
    // -------------------------------------------------------------------------
    let encrypted_vetkey = EncryptedVetKey::deserialize(&derive_response.encrypted_key)
        .expect("Failed to parse returned payload into EncryptedVetKey");

    // Decrypt and verify using the exact salted identity parameters
    let vetkey = encrypted_vetkey
        .decrypt_and_verify(&tsk, &dpk, &canonical_identity)
        .expect("Identity-Based verification and decryption of VetKey failed");

    let sk = vetkey.derive_symmetric_key("", 32);
    // -------------------------------------------------------------------------
    // STEP 4: Encrypt file
    // -------------------------------------------------------------------------
    let cipher = Aes256Gcm::new_from_slice(&sk)
        .expect("Failed to initialize AES-256-GCM cipher with derived key");

    let deterministic_nonce_bytes = Sha256::digest(entry_name.as_bytes());
    let nonce = aes_gcm::Nonce::from_slice(&deterministic_nonce_bytes[0..12]); // First 12 bytes

    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).unwrap();
    let file_size = ciphertext.len() as u64;
    println!("ciphertext length: {:?}", ciphertext.len());
    // -------------------------------------------------------------------------
    // STEP 4: Upload encrypted file
    // -------------------------------------------------------------------------
    // Keeping hash, plaintext_size, and file_size tracked to plaintext as requested.
    let hash_bytes = Sha256::digest(&ciphertext);
    let mut file_hash = [0u8; 32];
    file_hash.copy_from_slice(&hash_bytes);

    let init_args = core_nft_api::init_private_content_upload::Args {
        token_id_opt: None,
        plaintext_hash: plaintext_hash,
        file_hash: file_hash,
        salt: salt.clone(),
        entry_name: entry_name.clone(),
        default_readers: default_readers.clone(),
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        plaintext_size,
        expected_chunks: 1,
        chunk_size: Some(file_size), // Chunk size handles payload container dimensions
        file_size,
        encryption_mode: EncryptionMode::AES256,
    };

    let init_response =
        init_private_content_upload(pic, controller, collection_canister_id, &init_args);
    assert!(init_response.is_ok(), "init_private_content_upload failed");

    let store_response = store_private_content_chunk(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            plaintext_hash: plaintext_hash, // NOTE: encrypted chunk hash
            entry_name: entry_name.clone(),
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext.clone()), // Uploads the actual ciphertext
        },
    );
    println!("store_response: {:?}", store_response);
    assert!(store_response.is_ok(), "store_private_content_chunk failed");

    let finalize_response = finalize_private_content_upload(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            hash: plaintext_hash,
            entry_name: entry_name.clone(),
        },
    );
    println!("finalize_response: {:?}", finalize_response);
    assert!(
        finalize_response.is_ok(),
        "finalize_private_content_upload failed"
    );

    let private_entry = PrivateEntry {
        status: PrivateContentStatus::PendingMinting,
        readers: HashMap::new(),
        hash: plaintext_hash,
        salt: salt.clone(),
        plaintext_size,
        file_size,
        encryption_mode: EncryptionMode::AES256,
        canonical_identity: canonical_identity.to_vec().clone(),
        previous_canonical_identity: None,
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        pending_upload: None,
        format_version: 1,
    };

    let mint_response = mint(
        pic,
        controller,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(private_entry),
            }],
        },
    );
    assert!(mint_response.is_ok(), "Mint with private content failed");

    // -------------------------------------------------------------------------
    // STEP 4: Generate transport key
    // -------------------------------------------------------------------------
    let transport_seed = [101u8; 32]; // Differentiated from the first one
    let tsk = TransportSecretKey::from_seed(transport_seed.to_vec())
        .expect("Failed to initialize TransportSecretKey from seed");

    let tpk = tsk.public_key();

    // -------------------------------------------------------------------------
    // STEP 5: Request Decryption VetKey as the Anon Principal
    // -------------------------------------------------------------------------
    let derive_args = derive_vetkey::Args {
        input: ByteBuf::from(canonical_identity), // Requesting key specifically for this salted context
        transport_public_key: ByteBuf::from(tpk),
    };

    // Test with anonymous user
    let derive_response = derive_vetkey(
        pic,
        Principal::anonymous(),
        collection_canister_id,
        &derive_args,
    )
    .unwrap_err();

    // Final verification assertion
    assert_eq!(
        derive_response,
        "Caller is not part of the canonical identity"
    );
}
