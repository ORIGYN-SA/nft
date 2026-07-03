use crate::client::core_nft::*;
use crate::client::core_nft::{get_upload_status, grant_permission, mint};
use crate::core_suite::setup::default_test_setup;
use crate::core_suite::setup::setup::TestEnv;
use crate::core_suite::tests::test_private_content::mint::NftPrivateRecordMint;
use crate::utils::{setup_http_client, tick_n_blocks};
use aes_gcm::aead::Aead;
use aes_gcm::Aes256Gcm;
use aes_gcm::KeyInit;
use bity_ic_storage_canister_api::types::storage::UploadState;
use bytes::Bytes;
use candid::{Nat, Principal};
use core_nft_api::__get_private_entry_test::Args;
use core_nft_api::derive_vetkey;
use core_nft_api::set_readers;
use core_nft_common::types::management::{grant_permission, mint, mint::MintRequest};
use core_nft_common::types::permissions::Permission;
use core_nft_common::AccessRights;
use core_nft_common::EncryptionMode;
use core_nft_common::PrivateContentStatus;
use core_nft_common::PrivateEntry;
use core_nft_common::{construct_canonical_identity, ReaderInfo};
use http::Request;
use http_body_util::BodyExt;
use ic_cdk::println;
use ic_http_gateway::HttpGatewayRequestArgs;
use ic_vetkeys::DerivedPublicKey;
use ic_vetkeys::EncryptedVetKey;
use ic_vetkeys::TransportSecretKey;
use icrc_ledger_types::icrc1::account::Account;
use serde_bytes::ByteBuf;
use serde_json::{self, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::str::FromStr;

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

    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    )
    .unwrap();
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateUploads,
        }),
    )
    .unwrap();

    let content = b"0123456789abcdef".to_vec();
    let plaintext_size = content.len() as u64;
    let file_size = plaintext_size;
    let entry_name = "/private_test.bin".to_string();
    let storage_path = "/private/test.bin".to_string();

    let hash_bytes = Sha256::digest(&content);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);
    let salt = vec![];

    let readers = HashMap::new();
    let default_readers = HashMap::new();
    let init_args = core_nft_api::init_private_content_upload::Args {
        token_id_opt: None,
        entry_name: None,
        plaintext_hash: hash,
        file_hash: hash,
        salt: salt.clone(),
        readers: readers.clone(),
        default_readers: default_readers.clone(),
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        plaintext_size,
        expected_chunks: 1,
        chunk_size: Some(plaintext_size),
        file_size,
        encryption_mode: EncryptionMode::AES256,
    };

    let init_response =
        init_private_content_upload(pic, nft_owner1, collection_canister_id, &init_args);
    assert!(
        init_response.is_ok(),
        "init_private_content_upload failed: {:?}",
        init_response
    );

    let store_response = store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            storage_path: storage_path.clone(),
            plaintext_hash: hash,
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
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash,
            storage_path: storage_path.clone(),
        },
    );
    assert!(
        finalize_response.is_ok(),
        "finalize_private_content_upload failed: {:?}",
        finalize_response
    );

    let private_entry = PrivateEntry {
        status: PrivateContentStatus::PendingMinting,
        readers: readers.clone(),
        hash,
        salt,
        plaintext_size,
        file_size,
        encryption_mode: EncryptionMode::AES256,
        canonical_identity: construct_canonical_identity(nft_owner1, &readers, &default_readers),
        previous_canonical_identity: None,
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        pending_upload: None,
        format_version: 1,
    };

    let entries = HashMap::from([("test_file".to_string(), private_entry.hash)]);

    let mint_response = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: default_readers.clone(),
                    entries,
                }),
                public_content: None,
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
    let canonical_identity = construct_canonical_identity(nft_owner1, &readers, &default_readers);

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
        entry_name: None,
        plaintext_hash: plaintext_hash,
        file_hash: file_hash,
        salt: salt.clone(),
        readers: HashMap::new(),
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
        init_private_content_upload(pic, nft_owner1, collection_canister_id, &init_args);
    assert!(init_response.is_ok(), "init_private_content_upload failed");

    let store_response = store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            plaintext_hash: plaintext_hash, // NOTE: encrypted chunk hash
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext.clone()), // Uploads the actual ciphertext
            storage_path: storage_path.clone(),
        },
    );
    println!("store_response: {:?}", store_response);
    assert!(store_response.is_ok(), "store_private_content_chunk failed");

    let finalize_response = finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash: plaintext_hash,
            storage_path: storage_path.clone(),
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

    // 2. Put the entry into the map (HashMap::from takes an array of tuples)
    let entries = HashMap::from([("test_file".to_string(), private_entry.hash)]);

    let mint_response = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries,
                }),
                public_content: None,
            }],
        },
    )
    .unwrap();
    // assert!(mint_response.is_ok(), "Mint with private content failed");

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
    let readers = HashMap::new();
    let canonical_identity = construct_canonical_identity(nft_owner1, &readers, &default_readers);

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
    let pub_key_response = derive_vetkey_public_key(pic, nft_owner1, collection_canister_id, &())
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
        entry_name: None,
        plaintext_hash: plaintext_hash,
        file_hash: file_hash,
        salt: salt.clone(),
        readers: HashMap::new(),
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
        init_private_content_upload(pic, nft_owner1, collection_canister_id, &init_args);
    assert!(init_response.is_ok(), "init_private_content_upload failed");

    let store_response = store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            plaintext_hash: plaintext_hash, // NOTE: encrypted chunk hash
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext.clone()), // Uploads the actual ciphertext
            storage_path: storage_path.clone(),
        },
    );
    println!("store_response: {:?}", store_response);
    assert!(store_response.is_ok(), "store_private_content_chunk failed");

    let finalize_response = finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash: plaintext_hash,
            storage_path: storage_path.clone(),
        },
    );
    println!("finalize_response: {:?}", finalize_response);
    assert!(
        finalize_response.is_ok(),
        "finalize_private_content_upload failed"
    );

    let entries = HashMap::from([("test_file".to_string(), init_args.plaintext_hash)]);

    let mint_response = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries: entries.clone(),
                }),
                public_content: None,
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

#[test]
fn test_private_content_readers_access() {
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

    let reader_a = Principal::from_text("r7inp-6aaaa-aaaaa-aaabq-cai").unwrap();
    let readers: HashMap<_, _> = vec![(
        reader_a,
        ReaderInfo {
            rights: AccessRights::Read,
            alias: None,
        },
    )]
    .into_iter()
    .collect();
    let canonical_identity = construct_canonical_identity(nft_owner1, &readers, &default_readers); // NOTE: canonical identity is formed on the frontend side. If it's formed incorrectly - some readers might not have access to files

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
    let pub_key_response = derive_vetkey_public_key(pic, nft_owner1, collection_canister_id, &())
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
        entry_name: None,
        plaintext_hash: plaintext_hash,
        file_hash: file_hash,
        salt: salt.clone(),
        readers,
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
        init_private_content_upload(pic, nft_owner1, collection_canister_id, &init_args);
    assert!(init_response.is_ok(), "init_private_content_upload failed");

    let store_response = store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            plaintext_hash: plaintext_hash, // NOTE: encrypted chunk hash
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext.clone()), // Uploads the actual ciphertext
            storage_path: storage_path.clone(),
        },
    );
    println!("store_response: {:?}", store_response);
    assert!(store_response.is_ok(), "store_private_content_chunk failed");

    let finalize_response = finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash: plaintext_hash,
            storage_path: storage_path.clone(),
        },
    );
    println!("finalize_response: {:?}", finalize_response);
    assert!(
        finalize_response.is_ok(),
        "finalize_private_content_upload failed"
    );

    let entries = HashMap::from([("test_file".to_string(), init_args.plaintext_hash)]);

    let mint_response = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries: entries.clone(),
                }),
                public_content: None,
            }],
        },
    );
    println!("Mint: {:?}", mint_response);
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
    let derive_response = derive_vetkey(pic, reader_a, collection_canister_id, &derive_args);
    println!("derive_response: {:?}", derive_response);
    // Final verification assertion
    assert!(derive_response.is_ok());
}

#[test]
fn test_private_content_default_readers_access() {
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

    let readers = HashMap::new();
    let plaintext = b"Confidential NFT Asset Data 2026".to_vec();
    let plaintext_size = plaintext.len() as u64;
    let entry_name = "/test.txt".to_string();
    let storage_path = "/private/test.txt".to_string();

    let reader_a = Principal::from_text("r7inp-6aaaa-aaaaa-aaabq-cai").unwrap();
    let default_readers: HashMap<_, _> = vec![(
        reader_a,
        ReaderInfo {
            rights: AccessRights::Read,
            alias: None,
        },
    )]
    .into_iter()
    .collect();
    println!(
        "Owner: {:?}, controller: {:?}",
        nft_owner1.to_text(),
        controller.to_text()
    );
    let canonical_identity = construct_canonical_identity(nft_owner1, &readers, &default_readers); // NOTE: canonical identity is formed on the frontend side. If it's formed incorrectly

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
    let pub_key_response = derive_vetkey_public_key(pic, nft_owner1, collection_canister_id, &())
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
        entry_name: None,
        plaintext_hash: plaintext_hash,
        file_hash: file_hash,
        salt: salt.clone(),
        readers,
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
        init_private_content_upload(pic, nft_owner1, collection_canister_id, &init_args);
    assert!(init_response.is_ok(), "init_private_content_upload failed");

    let store_response = store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            plaintext_hash: plaintext_hash, // NOTE: encrypted chunk hash
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext.clone()), // Uploads the actual ciphertext
            storage_path: storage_path.clone(),
        },
    );
    println!("store_response: {:?}", store_response);
    assert!(store_response.is_ok(), "store_private_content_chunk failed");

    let finalize_response = finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash: plaintext_hash,
            storage_path: storage_path.clone(),
        },
    );
    println!("finalize_response: {:?}", finalize_response);
    assert!(
        finalize_response.is_ok(),
        "finalize_private_content_upload failed"
    );

    let entries = HashMap::from([("test_file".to_string(), init_args.plaintext_hash)]);

    let mint_response = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: default_readers.clone(),
                    entries: entries.clone(),
                }),
                public_content: None,
            }],
        },
    );
    println!("Mint :{:?}", mint_response);
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
    let derive_response = derive_vetkey(pic, reader_a, collection_canister_id, &derive_args);
    println!("derive_response: {:?}", derive_response);
    // Final verification assertion
    assert!(derive_response.is_ok());
}

#[test]
fn test_private_content_reencryption_workflow() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
        ..
    } = test_env;

    // Grant permissions to nft_owner1
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        },
    )
    .unwrap();
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateUploads,
        },
    )
    .unwrap();

    let entry_name = "reencryption_test".to_string();
    // let plaintext = b"Sensitive Data to be re-encrypted".to_vec();
    let plaintext = b"Confidential NFT Asset Data 2026".to_vec();
    let plaintext_hash_bytes = Sha256::digest(&plaintext);
    let mut plaintext_hash = [0u8; 32];
    plaintext_hash.copy_from_slice(&plaintext_hash_bytes);

    // 1. Initial Setup: Owner encrypts for themselves
    let mut initial_readers = HashMap::new();
    initial_readers.insert(
        nft_owner1,
        ReaderInfo {
            rights: AccessRights::ReadWriteManage,
            alias: None,
        },
    );
    let transport_seed = [102u8; 32];
    let tsk = TransportSecretKey::from_seed(transport_seed.to_vec())
        .expect("Failed to initialize TransportSecretKey from seed");

    let tpk = tsk.public_key();

    let pub_key_response = derive_vetkey_public_key(pic, controller, collection_canister_id, &())
        .expect("Canister returned an error deriving public key");

    let dpk = DerivedPublicKey::deserialize(&pub_key_response.public_key)
        .expect("Failed to deserialize public key using ic-vetkeys");

    let initial_canonical_identity =
        construct_canonical_identity(nft_owner1, &initial_readers, &HashMap::new());

    let derive_args = derive_vetkey::Args {
        input: ByteBuf::from(initial_canonical_identity.clone()),
        transport_public_key: ByteBuf::from(tpk),
    };
    let derive_response =
        derive_vetkey(pic, nft_owner1, collection_canister_id, &derive_args).unwrap();
    let vetkey = EncryptedVetKey::deserialize(&derive_response.encrypted_key)
        .unwrap()
        .decrypt_and_verify(&tsk, &dpk, &initial_canonical_identity)
        .unwrap();

    let sk = vetkey.derive_symmetric_key("", 32);
    let cipher = Aes256Gcm::new_from_slice(&sk)
        .expect("Failed to initialize AES-256-GCM cipher with derived key");

    let deterministic_nonce_bytes = Sha256::digest(entry_name.as_bytes());
    let nonce = aes_gcm::Nonce::from_slice(&deterministic_nonce_bytes[0..12]); // First 12 bytes

    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).unwrap();
    let file_size = ciphertext.len() as u64;
    println!("ciphertext length: {:?}", ciphertext.len());

    let hash_bytes = Sha256::digest(&ciphertext);
    let mut file_hash = [0u8; 32];
    file_hash.copy_from_slice(&hash_bytes);
    let salt = Sha256::digest(entry_name.as_bytes()).to_vec();

    // 2. Initial Upload
    let storage_path = "/private/initial_state.bin".to_string();
    init_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::init_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            plaintext_hash,
            file_hash,
            salt: salt.clone(),
            readers: HashMap::new(),
            default_readers: HashMap::new(),
            storage_canister_id: collection_canister_id,
            storage_path: storage_path.clone(),
            plaintext_size: plaintext.len() as u64,
            expected_chunks: 1,
            chunk_size: Some(file_size),
            file_size,
            encryption_mode: EncryptionMode::AES256,
        },
    )
    .unwrap();

    store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            plaintext_hash,
            storage_path: storage_path.clone(),
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext.clone()),
        },
    )
    .unwrap();

    finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash: plaintext_hash,
            storage_path: storage_path.clone(),
        },
    )
    .unwrap();

    // Assert upload state: Finalized (which maps to PendingMinting in the privacy system)
    assert_eq!(
        get_upload_status(pic, controller, collection_canister_id, &storage_path).unwrap(),
        UploadState::Finalized
    );

    // 3. Mint
    let mut entries = HashMap::new();
    entries.insert(entry_name.clone(), plaintext_hash);
    let token_id = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries: entries.clone(),
                }),
                public_content: None,
            }],
        },
    )
    .unwrap();
    tick_n_blocks(pic, 10);

    // Assert status: Active
    assert_eq!(
        __get_private_entry_test(
            pic,
            controller,
            collection_canister_id,
            &Args {
                token_id: token_id.clone(),
                entry_name: entry_name.clone()
            }
        )
        .unwrap()
        .status,
        PrivateContentStatus::Active
    );

    // 4. Update Readers (trigger PendingReencryption)
    let mut new_readers = HashMap::new();
    new_readers.insert(
        nft_owner2,
        ReaderInfo {
            rights: AccessRights::Read,
            alias: None,
        },
    );

    // Prepare for reader addition
    set_readers(
        pic,
        nft_owner1,
        collection_canister_id,
        &set_readers::Args {
            token_id: token_id.clone(),
            entry_name: entry_name.clone(),
            readers: new_readers.clone(),
        },
    )
    .unwrap();

    // Assert status: PendingReencryption
    assert_eq!(
        __get_private_entry_test(
            pic,
            controller,
            collection_canister_id,
            &Args {
                token_id: token_id.clone(),
                entry_name: entry_name.clone()
            }
        )
        .unwrap()
        .status,
        PrivateContentStatus::PendingReencryption
    );

    // 5. Re-encryption Upload (Overwrite)
    let new_canonical_identity =
        construct_canonical_identity(nft_owner1, &new_readers, &HashMap::new());

    let transport_seed_new = [101u8; 32];
    let tsk_new = TransportSecretKey::from_seed(transport_seed_new.to_vec()).unwrap();
    let tpk_new = tsk_new.public_key();

    // Derived key for the new identity
    let derive_args_new = derive_vetkey::Args {
        input: ByteBuf::from(new_canonical_identity.clone()),
        transport_public_key: ByteBuf::from(tpk_new),
    };
    let derive_response_new =
        derive_vetkey(pic, nft_owner1, collection_canister_id, &derive_args_new).unwrap();
    let vetkey_new = EncryptedVetKey::deserialize(&derive_response_new.encrypted_key)
        .unwrap()
        .decrypt_and_verify(&tsk_new, &dpk, &new_canonical_identity)
        .unwrap();

    let sk_new = vetkey_new.derive_symmetric_key("", 32);
    let cipher_new = Aes256Gcm::new_from_slice(&sk_new).unwrap();
    let ciphertext_new = cipher_new.encrypt(nonce, plaintext.as_ref()).unwrap();
    let file_size_new = ciphertext_new.len() as u64;
    let storage_path_new = storage_path.clone();

    let hash_bytes_new = Sha256::digest(&ciphertext_new);
    let mut file_hash_new = [0u8; 32];
    file_hash_new.copy_from_slice(&hash_bytes_new);

    init_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::init_private_content_upload::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            plaintext_hash,
            file_hash: file_hash_new,
            readers: HashMap::new(),
            salt: salt.clone(),
            default_readers: HashMap::new(),
            storage_canister_id: collection_canister_id,
            storage_path: storage_path_new.clone(),
            plaintext_size: plaintext.len() as u64,
            expected_chunks: 1,
            chunk_size: Some(file_size_new),
            file_size: file_size_new,
            encryption_mode: EncryptionMode::AES256,
        },
    )
    .unwrap();
    tick_n_blocks(pic, 10);

    // Assert upload state: Init (which maps to PendingUpload in the privacy system)
    assert_eq!(
        __get_private_entry_test(
            pic,
            controller,
            collection_canister_id,
            &Args {
                token_id: token_id.clone(),
                entry_name: entry_name.clone()
            }
        )
        .unwrap()
        .status,
        PrivateContentStatus::PendingReencryption
    );

    assert_eq!(
        get_upload_status(
            pic,
            controller,
            collection_canister_id,
            &storage_path.clone()
        )
        .unwrap(),
        UploadState::InitReupload
    );

    store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            plaintext_hash,
            storage_path: storage_path_new.clone(),
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext_new.clone()),
        },
    )
    .unwrap();

    finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            hash: plaintext_hash,
            storage_path: storage_path_new,
        },
    )
    .unwrap();

    // Assert status: Back to Active
    assert_eq!(
        __get_private_entry_test(
            pic,
            controller,
            collection_canister_id,
            &Args {
                token_id: token_id.clone(),
                entry_name: entry_name.clone()
            }
        )
        .unwrap()
        .status,
        PrivateContentStatus::Active
    );

    // 6. Verify accessibility for nft_owner2
    let derive_response_reader =
        derive_vetkey(pic, nft_owner2, collection_canister_id, &derive_args_new).unwrap();
    let vetkey_reader = EncryptedVetKey::deserialize(&derive_response_reader.encrypted_key)
        .unwrap()
        .decrypt_and_verify(&tsk_new, &dpk, &new_canonical_identity)
        .unwrap();

    let sk_reader = vetkey_reader.derive_symmetric_key("", 32);
    let cipher_reader = Aes256Gcm::new_from_slice(&sk_reader).unwrap();
    let decrypted = cipher_reader
        .decrypt(nonce, ciphertext_new.as_ref())
        .unwrap();

    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_private_content_transfer_workflow() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
        ..
    } = test_env;

    // Grant permissions to nft_owner1
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        },
    )
    .unwrap();
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateUploads,
        },
    )
    .unwrap();

    // Grant minting and uploads permission to nft_owner1
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner2,
            permission: Permission::Minting,
        }),
    )
    .unwrap();
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner2,
            permission: Permission::UpdateUploads,
        }),
    )
    .unwrap();

    let entry_name = "reencryption_test".to_string();
    let plaintext = b"Confidential NFT Asset Data 2026".to_vec();
    let plaintext_hash_bytes = Sha256::digest(&plaintext);
    let mut plaintext_hash = [0u8; 32];
    plaintext_hash.copy_from_slice(&plaintext_hash_bytes);

    let reader_a = Principal::from_text("r7inp-6aaaa-aaaaa-aaabq-cai").unwrap();
    let readers: HashMap<_, _> = vec![(
        reader_a,
        ReaderInfo {
            rights: AccessRights::Read,
            alias: None,
        },
    )]
    .into_iter()
    .collect();

    // 1. Initial Setup: Owner encrypts for themselves
    let mut initial_readers = HashMap::new();
    initial_readers.insert(
        nft_owner2,
        ReaderInfo {
            rights: AccessRights::ReadWriteManage,
            alias: None,
        },
    );
    let transport_seed = [102u8; 32];
    let tsk = TransportSecretKey::from_seed(transport_seed.to_vec())
        .expect("Failed to initialize TransportSecretKey from seed");
    let tpk = tsk.public_key();

    let pub_key_response = derive_vetkey_public_key(pic, controller, collection_canister_id, &())
        .expect("Canister returned an error deriving public key");

    let dpk = DerivedPublicKey::deserialize(&pub_key_response.public_key)
        .expect("Failed to deserialize public key using ic-vetkeys");

    let initial_canonical_identity =
        construct_canonical_identity(nft_owner1, &initial_readers, &HashMap::new());

    let derive_args = derive_vetkey::Args {
        input: ByteBuf::from(initial_canonical_identity.clone()),
        transport_public_key: ByteBuf::from(tpk.clone()),
    };
    let derive_response =
        derive_vetkey(pic, nft_owner1, collection_canister_id, &derive_args).unwrap();
    let vetkey = EncryptedVetKey::deserialize(&derive_response.encrypted_key)
        .unwrap()
        .decrypt_and_verify(&tsk, &dpk, &initial_canonical_identity)
        .unwrap();

    let sk = vetkey.derive_symmetric_key("", 32);
    let cipher = Aes256Gcm::new_from_slice(&sk)
        .expect("Failed to initialize AES-256-GCM cipher with derived key");

    let deterministic_nonce_bytes = Sha256::digest(entry_name.as_bytes());
    let nonce = aes_gcm::Nonce::from_slice(&deterministic_nonce_bytes[0..12]); // First 12 bytes

    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).unwrap();
    let file_size = ciphertext.len() as u64;
    println!("ciphertext length: {:?}", ciphertext.len());

    let hash_bytes = Sha256::digest(&ciphertext);
    let mut file_hash = [0u8; 32];
    file_hash.copy_from_slice(&hash_bytes);
    let salt = Sha256::digest(entry_name.as_bytes()).to_vec();

    // 2. Initial Upload
    let storage_path = "/private/initial_state.bin".to_string();
    init_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::init_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            plaintext_hash,
            file_hash,
            salt: salt.clone(),
            readers,
            default_readers: HashMap::new(),
            storage_canister_id: collection_canister_id,
            storage_path: storage_path.clone(),
            plaintext_size: plaintext.len() as u64,
            expected_chunks: 1,
            chunk_size: Some(file_size),
            file_size,
            encryption_mode: EncryptionMode::AES256,
        },
    )
    .unwrap();

    store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            plaintext_hash,
            storage_path: storage_path.clone(),
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext.clone()),
        },
    )
    .unwrap();

    finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash: plaintext_hash,
            storage_path: storage_path.clone(),
        },
    )
    .unwrap();

    // Assert upload state: Finalized (which maps to PendingMinting in the privacy system)
    assert_eq!(
        get_upload_status(pic, controller, collection_canister_id, &storage_path).unwrap(),
        UploadState::Finalized
    );

    // 3. Mint
    let mut entries = HashMap::new();
    entries.insert(entry_name.clone(), plaintext_hash);
    let token_id = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries: entries.clone(),
                }),
                public_content: None,
            }],
        },
    )
    .unwrap();
    tick_n_blocks(pic, 10);

    // Assert status: Active
    assert_eq!(
        __get_private_entry_test(
            pic,
            controller,
            collection_canister_id,
            &Args {
                token_id: token_id.clone(),
                entry_name: entry_name.clone()
            }
        )
        .unwrap()
        .status,
        PrivateContentStatus::Active
    );

    // 2. Transfer: Owner1 -> Owner2
    let _ = icrc7_transfer(
        pic,
        nft_owner1,
        collection_canister_id,
        &vec![core_nft_common::icrc7::TransferArg {
            to: Account {
                owner: nft_owner2,
                subaccount: None,
            },
            token_id: token_id.clone(),
            memo: None,
            from_subaccount: None,
            created_at_time: None,
        }],
    );

    // 3. Assert
    assert_eq!(
        __get_private_entry_test(
            pic,
            controller,
            collection_canister_id,
            &Args {
                token_id: token_id.clone(),
                entry_name: entry_name.clone()
            }
        )
        .unwrap()
        .status,
        PrivateContentStatus::PendingReencryption
    );

    // 5. Re-encryption Upload (Overwrite)
    let new_canonical_identity =
        construct_canonical_identity(nft_owner2, &HashMap::new(), &HashMap::new()); // share to nobody

    let transport_seed_owner_2 = [201u8; 32];
    let tsk_owner_2 = TransportSecretKey::from_seed(transport_seed_owner_2.to_vec()).unwrap();
    let tpk_owner_2 = tsk_owner_2.public_key();

    // Derived key for the new identity
    let derive_args_new = derive_vetkey::Args {
        input: ByteBuf::from(new_canonical_identity.clone()),
        transport_public_key: ByteBuf::from(tpk_owner_2),
    };
    let derive_response_new =
        derive_vetkey(pic, nft_owner2, collection_canister_id, &derive_args_new).unwrap();
    let vetkey_new = EncryptedVetKey::deserialize(&derive_response_new.encrypted_key)
        .unwrap()
        .decrypt_and_verify(&tsk_owner_2, &dpk, &new_canonical_identity)
        .unwrap();

    let sk_new = vetkey_new.derive_symmetric_key("", 32);
    let cipher_new = Aes256Gcm::new_from_slice(&sk_new).unwrap();
    let ciphertext_new = cipher_new.encrypt(nonce, plaintext.as_ref()).unwrap();
    let file_size_new = ciphertext_new.len() as u64;

    let hash_bytes_new = Sha256::digest(&ciphertext_new);
    let mut file_hash_new = [0u8; 32];
    file_hash_new.copy_from_slice(&hash_bytes_new);

    let storage_path_reupload = format!("/private/initial_state.bin");
    let unauth_upload = init_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::init_private_content_upload::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            plaintext_hash,
            file_hash: file_hash_new,
            salt: salt.clone(),
            readers: HashMap::new(),
            default_readers: HashMap::new(),
            storage_canister_id: collection_canister_id,
            storage_path: storage_path_reupload.clone(),
            plaintext_size: plaintext.len() as u64,
            expected_chunks: 1,
            chunk_size: Some(file_size_new),
            file_size: file_size_new,
            encryption_mode: EncryptionMode::AES256,
        },
    );
    assert!(unauth_upload.is_err());

    init_private_content_upload(
        pic,
        nft_owner2,
        collection_canister_id,
        &core_nft_api::init_private_content_upload::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            plaintext_hash,
            file_hash: file_hash_new,
            salt: salt.clone(),
            readers: HashMap::new(),
            default_readers: HashMap::new(),
            storage_canister_id: collection_canister_id,
            storage_path: storage_path_reupload.clone(),
            plaintext_size: plaintext.len() as u64,
            expected_chunks: 1,
            chunk_size: Some(file_size_new),
            file_size: file_size_new,
            encryption_mode: EncryptionMode::AES256,
        },
    )
    .unwrap();
    tick_n_blocks(pic, 10);

    // Assert upload state: Init (which maps to PendingUpload in the privacy system)
    assert_eq!(
        __get_private_entry_test(
            pic,
            controller,
            collection_canister_id,
            &Args {
                token_id: token_id.clone(),
                entry_name: entry_name.clone()
            }
        )
        .unwrap()
        .status,
        PrivateContentStatus::PendingReencryption
    );
    tick_n_blocks(pic, 10);

    store_private_content_chunk(
        pic,
        nft_owner2,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            plaintext_hash,
            storage_path: storage_path_reupload.clone(),
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext_new.clone()),
        },
    )
    .unwrap();

    finalize_private_content_upload(
        pic,
        nft_owner2,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            hash: plaintext_hash,
            storage_path: storage_path_reupload,
        },
    )
    .unwrap();

    // Assert status: Back to Active
    assert_eq!(
        __get_private_entry_test(
            pic,
            controller,
            collection_canister_id,
            &Args {
                token_id: token_id.clone(),
                entry_name: entry_name.clone()
            }
        )
        .unwrap()
        .status,
        PrivateContentStatus::Active
    );

    // 6. Verify accessibility for nft_owner2
    let derive_response_reader =
        derive_vetkey(pic, nft_owner2, collection_canister_id, &derive_args_new).unwrap();
    let vetkey_reader = EncryptedVetKey::deserialize(&derive_response_reader.encrypted_key)
        .unwrap()
        .decrypt_and_verify(&tsk_owner_2, &dpk, &new_canonical_identity)
        .unwrap();

    let sk_reader = vetkey_reader.derive_symmetric_key("", 32);
    let cipher_reader = Aes256Gcm::new_from_slice(&sk_reader).unwrap();
    let decrypted = cipher_reader
        .decrypt(nonce, ciphertext_new.as_ref())
        .unwrap();

    assert_eq!(decrypted, plaintext);

    // test whether owner 1 has access
    let derive_args_new = derive_vetkey::Args {
        input: ByteBuf::from(new_canonical_identity.clone()),
        transport_public_key: ByteBuf::from(tpk.clone()),
    };
    let derive_response_new =
        derive_vetkey(pic, nft_owner1, collection_canister_id, &derive_args_new);
    assert!(derive_response_new.is_err());

    let derive_args = derive_vetkey::Args {
        input: ByteBuf::from(new_canonical_identity.clone()), // Requesting key specifically for this salted context
        transport_public_key: ByteBuf::from(tpk),
    };

    // Test with anonymous user
    let derive_response_new_1 = derive_vetkey(pic, reader_a, collection_canister_id, &derive_args);
    assert!(derive_response_new_1.is_err());
}

// FIXME: ideally we need to decline such interceptions
#[test]
fn test_private_content_upload_intercept() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;

    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    )
    .unwrap();
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateUploads,
        }),
    )
    .unwrap();

    let content = b"0123456789abcdef".to_vec();
    let plaintext_size = content.len() as u64;
    let file_size = plaintext_size;
    let entry_name = "/private_test.bin".to_string();
    let storage_path = "/private/test.bin".to_string();

    let hash_bytes = Sha256::digest(&content);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);
    let salt = vec![];

    let readers = HashMap::new();
    let default_readers = HashMap::new();
    let init_args = core_nft_api::init_private_content_upload::Args {
        token_id_opt: None,
        entry_name: None,
        plaintext_hash: hash,
        file_hash: hash,
        salt: salt.clone(),
        readers: readers.clone(),
        default_readers: default_readers.clone(),
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        plaintext_size,
        expected_chunks: 1,
        chunk_size: Some(plaintext_size),
        file_size,
        encryption_mode: EncryptionMode::AES256,
    };

    let init_response =
        init_private_content_upload(pic, nft_owner1, collection_canister_id, &init_args);
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
            entry_name: None,
            storage_path: storage_path.clone(),
            plaintext_hash: hash,
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
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash,
            storage_path: storage_path.clone(),
        },
    );
    assert!(
        finalize_response.is_ok(),
        "finalize_private_content_upload failed: {:?}",
        finalize_response
    );

    let private_entry = PrivateEntry {
        status: PrivateContentStatus::PendingMinting,
        readers: readers.clone(),
        hash,
        salt,
        plaintext_size,
        file_size,
        encryption_mode: EncryptionMode::AES256,
        canonical_identity: construct_canonical_identity(nft_owner1, &readers, &default_readers),
        previous_canonical_identity: None,
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        pending_upload: None,
        format_version: 1,
    };

    let entries = HashMap::from([("test_file".to_string(), private_entry.hash)]);

    let mint_response = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: default_readers.clone(),
                    entries,
                }),
                public_content: None,
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
fn test_private_content_burn() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;

    // Grant minting and uploads permission to nft_owner1
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    )
    .unwrap();
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateUploads,
        }),
    )
    .unwrap();

    // 1. Upload private content as nft_owner1
    let content = b"supersecretdata!".to_vec();
    let plaintext_size = content.len() as u64;
    let file_size = plaintext_size;
    let entry_name = "/private_test_security.bin".to_string();
    let storage_path = "/private/test_security.bin".to_string();

    let hash_bytes = Sha256::digest(&content);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);
    let salt = vec![];

    let readers = HashMap::new();
    let default_readers = HashMap::new();

    let init_args = core_nft_api::init_private_content_upload::Args {
        token_id_opt: None,
        entry_name: None,
        plaintext_hash: hash,
        file_hash: hash,
        salt: salt.clone(),
        readers: readers.clone(),
        default_readers: default_readers.clone(),
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        plaintext_size,
        expected_chunks: 1,
        chunk_size: Some(plaintext_size),
        file_size,
        encryption_mode: EncryptionMode::AES256,
    };

    init_private_content_upload(pic, nft_owner1, collection_canister_id, &init_args).unwrap();
    store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            storage_path: storage_path.clone(),
            plaintext_hash: hash,
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(content.clone()),
        },
    )
    .unwrap();
    finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash,
            storage_path: storage_path.clone(),
        },
    )
    .unwrap();

    let private_entry = PrivateEntry {
        status: PrivateContentStatus::PendingMinting,
        readers: readers.clone(),
        hash,
        salt,
        plaintext_size,
        file_size,
        encryption_mode: EncryptionMode::AES256,
        canonical_identity: construct_canonical_identity(nft_owner1, &readers, &default_readers),
        previous_canonical_identity: None,
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        pending_upload: None,
        format_version: 1,
    };

    let entries = HashMap::from([("test_file".to_string(), private_entry.hash)]);

    // Mint the NFT
    let mint_result = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: default_readers.clone(),
                    entries,
                }),
                public_content: None,
            }],
        },
    );
    let token_id = mint_result.unwrap();

    // Verify default reader nft_owner2 is set
    let entry_info = __get_private_entry_test(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::__get_private_entry_test::Args {
            token_id: token_id.clone(),
            entry_name: "test_file".to_string(),
        },
    )
    .unwrap();
    assert_eq!(entry_info.status, PrivateContentStatus::Active);

    // 6. Burn NFT deletes the private file
    // Let's burn the NFT (caller must be the new owner, nft_owner1)
    burn_nft(pic, nft_owner1, collection_canister_id, &token_id).unwrap();

    // Verify private entry query returns Err (NotFound)
    let entry_info_after_burn = __get_private_entry_test(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::__get_private_entry_test::Args {
            token_id: token_id.clone(),
            entry_name: "test_file".to_string(),
        },
    );
    assert!(entry_info_after_burn.is_err());
}

#[test]
fn test_private_content_security_requirements() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    // Grant minting and uploads permission to nft_owner1
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    )
    .unwrap();
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateUploads,
        }),
    )
    .unwrap();
    // Grant minting and uploads permission to nft_owner2
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner2,
            permission: Permission::Minting,
        }),
    )
    .unwrap();
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner2,
            permission: Permission::UpdateUploads,
        }),
    )
    .unwrap();

    // 1. Upload private content as nft_owner1
    let content = b"supersecretdata!".to_vec();
    let plaintext_size = content.len() as u64;
    let file_size = plaintext_size;
    let entry_name = "/private_test_security.bin".to_string();
    let storage_path = "/private/test_security.bin".to_string();

    let hash_bytes = Sha256::digest(&content);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);
    let salt = vec![];

    let readers = HashMap::new();

    // Add default_readers: nft_owner2 is a default reader
    let mut default_readers = HashMap::new();
    default_readers.insert(
        nft_owner2,
        ReaderInfo {
            rights: AccessRights::Read,
            alias: None,
        },
    );

    let init_args = core_nft_api::init_private_content_upload::Args {
        token_id_opt: None,
        entry_name: None,
        plaintext_hash: hash,
        file_hash: hash,
        salt: salt.clone(),
        readers: readers.clone(),
        default_readers: default_readers.clone(),
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        plaintext_size,
        expected_chunks: 1,
        chunk_size: Some(plaintext_size),
        file_size,
        encryption_mode: EncryptionMode::AES256,
    };

    init_private_content_upload(pic, nft_owner1, collection_canister_id, &init_args).unwrap();
    store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            storage_path: storage_path.clone(),
            plaintext_hash: hash,
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(content.clone()),
        },
    )
    .unwrap();
    finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash,
            storage_path: storage_path.clone(),
        },
    )
    .unwrap();

    let private_entry = PrivateEntry {
        status: PrivateContentStatus::PendingMinting,
        readers: readers.clone(),
        hash,
        salt,
        plaintext_size,
        file_size,
        encryption_mode: EncryptionMode::AES256,
        canonical_identity: construct_canonical_identity(nft_owner1, &readers, &default_readers),
        previous_canonical_identity: None,
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        pending_upload: None,
        format_version: 1,
    };

    let entries = HashMap::from([("test_file".to_string(), private_entry.hash)]);

    // Mint the NFT
    let mint_result = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: default_readers.clone(),
                    entries,
                }),
                public_content: None,
            }],
        },
    );
    let token_id = mint_result.unwrap();

    // Verify default reader nft_owner2 is set
    let entry_info = __get_private_entry_test(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::__get_private_entry_test::Args {
            token_id: token_id.clone(),
            entry_name: "test_file".to_string(),
        },
    )
    .unwrap();
    assert_eq!(entry_info.status, PrivateContentStatus::Active);

    // 2. Set an entry reader with ReadWrite (ReadAndUpdate) rights
    let mut new_readers = HashMap::new();
    new_readers.insert(
        nft_owner2,
        ReaderInfo {
            rights: AccessRights::ReadWrite,
            alias: None,
        },
    );
    set_readers(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::set_readers::Args {
            token_id: token_id.clone(),
            entry_name: "test_file".to_string(),
            readers: new_readers.clone(),
        },
    )
    .unwrap();

    // Verify entry is now PendingReencryption
    let entry_info_pending = __get_private_entry_test(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::__get_private_entry_test::Args {
            token_id: token_id.clone(),
            entry_name: "test_file".to_string(),
        },
    )
    .unwrap();
    assert_eq!(
        entry_info_pending.status,
        PrivateContentStatus::PendingReencryption
    );

    // 3. Reader with ReadWrite (ReadAndUpdate) rights (nft_owner2) reencrypts and reuploads!
    let storage_path_new = storage_path.clone();
    let re_init_args = core_nft_api::init_private_content_upload::Args {
        token_id_opt: Some(token_id.clone()),
        entry_name: Some("test_file".to_string()),
        plaintext_hash: hash,
        file_hash: hash,
        salt: vec![],
        readers: HashMap::new(),
        default_readers: HashMap::new(),
        storage_canister_id: collection_canister_id,
        storage_path: storage_path_new.clone(),
        plaintext_size,
        expected_chunks: 1,
        chunk_size: Some(plaintext_size),
        file_size,
        encryption_mode: EncryptionMode::AES256,
    };
    // Call as nft_owner2 (who does NOT have Permission::UpdateUploads but is a reader with ReadWrite access)
    init_private_content_upload(pic, nft_owner2, collection_canister_id, &re_init_args).unwrap();

    store_private_content_chunk(
        pic,
        nft_owner2,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some("test_file".to_string()),
            storage_path: storage_path_new.clone(),
            plaintext_hash: hash,
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(content.clone()),
        },
    )
    .unwrap();

    finalize_private_content_upload(
        pic,
        nft_owner2,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some("test_file".to_string()),
            hash,
            storage_path: storage_path_new.clone(),
        },
    )
    .unwrap();

    // Verify entry status is active again
    let entry_info_active = __get_private_entry_test(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::__get_private_entry_test::Args {
            token_id: token_id.clone(),
            entry_name: "test_file".to_string(),
        },
    )
    .unwrap();
    assert_eq!(entry_info_active.status, PrivateContentStatus::Active);

    // 4. Spender token approval test
    let spender_acc = Account {
        owner: nft_owner2,
        subaccount: None,
    };
    let approve_args = vec![
        core_nft_common::types::icrc37::icrc37_approve_tokens::ApproveTokenArg {
            token_id: token_id.clone(),
            approval_info: core_nft_common::types::icrc37::ApprovalInfo {
                from_subaccount: None,
                spender: spender_acc.clone(),
                expires_at: None,
                memo: None,
                created_at_time: 0,
            },
        },
    ];
    icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args).unwrap();

    // Approved spender (nft_owner2) tries to transfer
    let xfer_args = vec![
        core_nft_common::types::icrc37::icrc37_transfer_from::TransferFromArg {
            spender_subaccount: None,
            from: Account {
                owner: nft_owner1,
                subaccount: None,
            },
            to: Account {
                owner: nft_owner2,
                subaccount: None,
            },
            token_id: token_id.clone(),
            memo: None,
            created_at_time: None,
        },
    ];
    let xfer_res =
        icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &xfer_args).unwrap();
    // Verify it is rejected due to transfer restriction on private content NFTs (returns Err)
    assert!(
        matches!(
            xfer_res[0].as_ref().unwrap(),
            core_nft_common::types::icrc37::icrc37_transfer_from::TransferFromResult::Err(
                core_nft_common::types::icrc37::icrc37_transfer_from::TransferFromError::Unauthorized
            )
        ),
        "Transfer by delegate should be unauthorized"
    );

    // 5. Transfer by owner:
    // Owner transfers directly.
    let owner_xfer_args = core_nft_common::types::icrc7::icrc7_transfer::Args::from(vec![
        core_nft_common::types::icrc7::TransferArg {
            from_subaccount: None,
            to: Account {
                owner: nft_owner2,
                subaccount: None,
            },
            token_id: token_id.clone(),
            memo: None,
            created_at_time: None,
        },
    ]);
    let owner_xfer_res = icrc7_transfer(pic, nft_owner1, collection_canister_id, &owner_xfer_args);
    assert!(owner_xfer_res[0].as_ref().unwrap().is_ok());

    // Verify readers reset (entry readers cleared, default readers preserved, new owner can set new readers)
    let entry_info_after_xfer = __get_private_entry_test(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::__get_private_entry_test::Args {
            token_id: token_id.clone(),
            entry_name: "test_file".to_string(),
        },
    )
    .unwrap();
    // Entry readers should be empty, status should be PendingReencryption
    assert!(entry_info_after_xfer.readers.is_empty());
    assert_eq!(
        entry_info_after_xfer.status,
        PrivateContentStatus::PendingReencryption
    );

    // 6. Burn NFT deletes the private file
    // Let's burn the NFT (caller must be the new owner, nft_owner2)
    burn_nft(pic, nft_owner2, collection_canister_id, &token_id).unwrap();

    // Verify private entry query returns Err (NotFound)
    let entry_info_after_burn = __get_private_entry_test(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::__get_private_entry_test::Args {
            token_id: token_id.clone(),
            entry_name: "test_file".to_string(),
        },
    );
    assert!(entry_info_after_burn.is_err());
}

#[test]
fn test_private_content_download() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;

    // 1. Grant permissions
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    )
    .unwrap();
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateUploads,
        }),
    )
    .unwrap();

    // 2. Upload private content as nft_owner1
    let content = b"supersecretpayload12345678901234".to_vec(); // 32 bytes (multiple of 16 for AES block size)
    let plaintext_size = content.len() as u64;
    let file_size = plaintext_size;
    let _entry_name = "/download_test.bin".to_string();
    let storage_path = "/private/download_test.bin".to_string();

    let hash_bytes = Sha256::digest(&content);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);
    let salt = vec![];

    let readers = HashMap::new();
    let default_readers = HashMap::new();

    let init_args = core_nft_api::init_private_content_upload::Args {
        token_id_opt: None,
        entry_name: None,
        plaintext_hash: hash,
        file_hash: hash,
        salt: salt.clone(),
        readers: readers.clone(),
        default_readers: default_readers.clone(),
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        plaintext_size,
        expected_chunks: 1,
        chunk_size: Some(plaintext_size),
        file_size,
        encryption_mode: EncryptionMode::AES256,
    };

    init_private_content_upload(pic, nft_owner1, collection_canister_id, &init_args).unwrap();
    store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            storage_path: storage_path.clone(),
            plaintext_hash: hash,
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(content.clone()),
        },
    )
    .unwrap();
    finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash,
            storage_path: storage_path.clone(),
        },
    )
    .unwrap();

    let private_entry = PrivateEntry {
        status: PrivateContentStatus::PendingMinting,
        readers: readers.clone(),
        hash,
        salt,
        plaintext_size,
        file_size,
        encryption_mode: EncryptionMode::AES256,
        canonical_identity: construct_canonical_identity(nft_owner1, &readers, &default_readers),
        previous_canonical_identity: None,
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        pending_upload: None,
        format_version: 1,
    };

    let entries = HashMap::from([("test_entry".to_string(), private_entry.hash)]);

    // Mint the NFT
    let mint_result = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: default_readers.clone(),
                    entries,
                }),
                public_content: None,
            }],
        },
    );
    let token_id = mint_result.unwrap();

    // 3. Fetch private content info as nft_owner1 (authorized) and verify it has the entry info & path
    let entry_info = __get_private_entry_test(
        pic,
        nft_owner1, // authorized caller (owner)
        collection_canister_id,
        &core_nft_api::__get_private_entry_test::Args {
            token_id: token_id.clone(),
            entry_name: "test_entry".to_string(),
        },
    )
    .unwrap();

    assert_eq!(entry_info.status, PrivateContentStatus::Active);
    assert_eq!(entry_info.storage_path, storage_path);

    // 4. Try fetching as controller (debug caller, also authorized)
    let entry_info_controller = __get_private_entry_test(
        pic,
        controller, // canister controller (authorized)
        collection_canister_id,
        &core_nft_api::__get_private_entry_test::Args {
            token_id: token_id.clone(),
            entry_name: "test_entry".to_string(),
        },
    )
    .unwrap();
    assert_eq!(entry_info_controller.status, PrivateContentStatus::Active);

    // 5. Try fetching as random user (unauthorized) and verify it returns Err
    let unauthorized_caller = Principal::anonymous();
    let unauthorized_result = __get_private_entry_test(
        pic,
        unauthorized_caller,
        collection_canister_id,
        &core_nft_api::__get_private_entry_test::Args {
            token_id: token_id.clone(),
            entry_name: "test_entry".to_string(),
        },
    );
    assert!(
        unauthorized_result.is_err(),
        "Unauthorized caller should not be able to retrieve private entry"
    );

    // 6. Download content using the HTTP gateway by following canister redirection
    let (rt, http_gateway) = setup_http_client(pic);

    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(entry_info.storage_path.clone())
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    assert_eq!(
        response.canister_response.status(),
        307,
        "Initial request must redirect"
    );

    if let Some(location) = response.canister_response.headers().get("location") {
        let location_str = location.to_str().unwrap();
        println!("Redirecting private content to: {}", location_str);

        // Extract subcanister ID from redirect URI
        let http_prefix = "http://";
        let subcanister_id = Principal::from_str(
            location_str
                .split('.')
                .next()
                .unwrap()
                .replace(http_prefix, "")
                .as_str(),
        )
        .unwrap();

        let redirected_response = rt.block_on(async {
            http_gateway
                .request(HttpGatewayRequestArgs {
                    canister_id: subcanister_id,
                    canister_request: Request::builder()
                        .uri(location_str)
                        .body(Bytes::new())
                        .unwrap(),
                })
                .send()
                .await
        });

        let mut final_response = redirected_response;
        if final_response.canister_response.status() == 307 {
            let location_bis = final_response
                .canister_response
                .headers()
                .get("location")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            println!("Redirecting again to: {}", location_bis);
            final_response = rt.block_on(async {
                http_gateway
                    .request(HttpGatewayRequestArgs {
                        canister_id: subcanister_id,
                        canister_request: Request::builder()
                            .uri(location_bis)
                            .body(Bytes::new())
                            .unwrap(),
                    })
                    .send()
                    .await
            });
        }

        assert_eq!(
            final_response.canister_response.status(),
            200,
            "Redirected request must return 200 OK"
        );

        let downloaded_body = rt.block_on(async {
            final_response
                .canister_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec()
        });

        assert_eq!(
            downloaded_body, content,
            "Downloaded content must match uploaded content"
        );
    } else {
        panic!("No location header found in redirection response");
    }
}

#[test]
fn test_private_content_reencryption_edgecases() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;

    // Grant permissions to nft_owner1
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        },
    )
    .unwrap();
    grant_permission(
        pic,
        controller,
        collection_canister_id,
        &grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateUploads,
        },
    )
    .unwrap();

    let entry_name = "reencryption_edge_test".to_string();
    let plaintext = b"Edge Case Data 2".to_vec();
    let plaintext_hash_bytes = Sha256::digest(&plaintext);
    let mut plaintext_hash = [0u8; 32];
    plaintext_hash.copy_from_slice(&plaintext_hash_bytes);

    let mut initial_readers = HashMap::new();
    initial_readers.insert(
        nft_owner1,
        ReaderInfo {
            rights: AccessRights::ReadWriteManage,
            alias: None,
        },
    );
    let transport_seed = [102u8; 32];
    let tsk = TransportSecretKey::from_seed(transport_seed.to_vec()).unwrap();
    let tpk = tsk.public_key();

    let pub_key_response =
        derive_vetkey_public_key(pic, controller, collection_canister_id, &()).unwrap();
    let dpk = DerivedPublicKey::deserialize(&pub_key_response.public_key).unwrap();

    let initial_canonical_identity =
        construct_canonical_identity(nft_owner1, &initial_readers, &HashMap::new());

    let derive_args = derive_vetkey::Args {
        input: ByteBuf::from(initial_canonical_identity.clone()),
        transport_public_key: ByteBuf::from(tpk.clone()),
    };
    let derive_response =
        derive_vetkey(pic, nft_owner1, collection_canister_id, &derive_args).unwrap();
    let vetkey = EncryptedVetKey::deserialize(&derive_response.encrypted_key)
        .unwrap()
        .decrypt_and_verify(&tsk, &dpk, &initial_canonical_identity)
        .unwrap();

    let sk = vetkey.derive_symmetric_key("", 32);
    let cipher = Aes256Gcm::new_from_slice(&sk).unwrap();
    let nonce = aes_gcm::Nonce::from_slice(&[0u8; 12]);
    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).unwrap();
    let file_size = ciphertext.len() as u64;

    let hash_bytes = Sha256::digest(&ciphertext);
    let mut file_hash = [0u8; 32];
    file_hash.copy_from_slice(&hash_bytes);
    let salt = vec![];

    let storage_path = "/private/edge_test.bin".to_string();

    // 1. Initial Upload
    init_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::init_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            plaintext_hash,
            file_hash,
            readers: initial_readers.clone(),
            salt: salt.clone(),
            default_readers: HashMap::new(),
            storage_canister_id: collection_canister_id,
            storage_path: storage_path.clone(),
            plaintext_size: plaintext.len() as u64,
            expected_chunks: 1,
            chunk_size: Some(file_size),
            file_size,
            encryption_mode: EncryptionMode::AES256,
        },
    )
    .unwrap();

    store_private_content_chunk(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::store_private_content_chunk::Args {
            token_id_opt: None,
            entry_name: None,
            storage_path: storage_path.clone(),
            plaintext_hash,
            chunk_index: Nat::from(0u64),
            chunk_data: ByteBuf::from(ciphertext.clone()),
        },
    )
    .unwrap();

    finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash: plaintext_hash,
            storage_path: storage_path.clone(),
        },
    )
    .unwrap();

    // Mint NFT
    let mut entries = HashMap::new();
    entries.insert(entry_name.clone(), plaintext_hash);
    let token_id = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries: entries.clone(),
                }),
                public_content: None,
            }],
        },
    )
    .unwrap();

    // 2. Test Edge Case: Plaintext Hash Mismatch
    let mut wrong_plaintext_hash = plaintext_hash;
    wrong_plaintext_hash[0] ^= 1; // Corrupt first byte

    let hash_mismatch_res = init_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::init_private_content_upload::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            plaintext_hash: wrong_plaintext_hash,
            file_hash,
            readers: initial_readers.clone(),
            salt: salt.clone(),
            default_readers: HashMap::new(),
            storage_canister_id: collection_canister_id,
            storage_path: storage_path.clone(),
            plaintext_size: plaintext.len() as u64,
            expected_chunks: 1,
            chunk_size: Some(file_size),
            file_size,
            encryption_mode: EncryptionMode::AES256,
        },
    );
    assert!(
        hash_mismatch_res.is_err(),
        "Re-upload with mismatched plaintext hash should fail"
    );

    // 3. Test Edge Case: Plaintext Size Mismatch
    let size_mismatch_res = init_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::init_private_content_upload::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            plaintext_hash,
            file_hash,
            readers: initial_readers.clone(),
            salt: salt.clone(),
            default_readers: HashMap::new(),
            storage_canister_id: collection_canister_id,
            storage_path: storage_path.clone(),
            plaintext_size: (plaintext.len() + 10) as u64, // wrong size
            expected_chunks: 1,
            chunk_size: Some(file_size),
            file_size,
            encryption_mode: EncryptionMode::AES256,
        },
    );
    assert!(
        size_mismatch_res.is_err(),
        "Re-upload with mismatched plaintext size should fail"
    );

    let unauthorized_caller = Principal::anonymous();
    let _grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: unauthorized_caller,
            permission: Permission::Minting,
        }),
    );
    let _grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: unauthorized_caller,
            permission: Permission::UpdateUploads,
        }),
    );
    let unauthorized_res = init_private_content_upload(
        pic,
        unauthorized_caller,
        collection_canister_id,
        &core_nft_api::init_private_content_upload::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            plaintext_hash,
            file_hash,
            readers: initial_readers.clone(),
            salt: salt.clone(),
            default_readers: HashMap::new(),
            storage_canister_id: collection_canister_id,
            storage_path: storage_path.clone(),
            plaintext_size: plaintext.len() as u64,
            expected_chunks: 1,
            chunk_size: Some(file_size),
            file_size,
            encryption_mode: EncryptionMode::AES256,
        },
    );
    assert!(
        unauthorized_res.is_err(),
        "Re-upload by unauthorized caller should fail"
    );

    // 5. Test Edge Case: Finalize with incorrect path
    let finalize_wrong_path = finalize_private_content_upload(
        pic,
        nft_owner1,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: Some(token_id.clone()),
            entry_name: Some(entry_name.clone()),
            hash: plaintext_hash,
            storage_path: "/private/non_existent_path.bin".to_string(),
        },
    );
    assert!(
        finalize_wrong_path.is_err(),
        "Finalize with non-existent path should fail"
    );
}
