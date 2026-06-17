use crate::client::core_nft::*;
use crate::client::core_nft::{get_upload_status, grant_permission, mint};
use crate::core_suite::setup::default_test_setup;
use crate::core_suite::setup::setup::TestEnv;
use crate::core_suite::tests::test_private_content::mint::NftPrivateRecordMint;
use crate::utils::tick_n_blocks;
use aes_gcm::aead::Aead;
use aes_gcm::Aes256Gcm;
use aes_gcm::KeyInit;
use bity_ic_storage_canister_api::types::storage::UploadState;
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
use ic_cdk::println;
use ic_vetkeys::DerivedPublicKey;
use ic_vetkeys::EncryptedVetKey;
use ic_vetkeys::TransportSecretKey;
use icrc_ledger_types::icrc1::account::Account;
use serde_bytes::ByteBuf;
use serde_json::{self, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const REUPLOAD_PREFIX: &str = "__reupload__:";

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
        entry_name: None,
        plaintext_hash: hash,
        file_hash: hash,
        salt: salt.clone(),
        readers: HashMap::new(),
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
        controller,
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
        readers: HashMap::new(),
        hash,
        salt,
        plaintext_size,
        file_size,
        encryption_mode: EncryptionMode::AES256,
        canonical_identity: construct_canonical_identity(
            Principal::anonymous(),
            &HashMap::new(),
            &HashMap::new(),
        ),
        previous_canonical_identity: None,
        storage_canister_id: collection_canister_id,
        storage_path: storage_path.clone(),
        pending_upload: None,
        format_version: 1,
    };

    let entries = HashMap::from([("test_file".to_string(), private_entry.hash)]);

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
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries,
                }),
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
    let canonical_identity =
        construct_canonical_identity(Principal::anonymous(), &readers, &default_readers);

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
        init_private_content_upload(pic, controller, collection_canister_id, &init_args);
    assert!(init_response.is_ok(), "init_private_content_upload failed");

    let store_response = store_private_content_chunk(
        pic,
        controller,
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
        controller,
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
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries,
                }),
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
        init_private_content_upload(pic, controller, collection_canister_id, &init_args);
    assert!(init_response.is_ok(), "init_private_content_upload failed");

    let store_response = store_private_content_chunk(
        pic,
        controller,
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
        controller,
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
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries: entries.clone(),
                }),
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
        init_private_content_upload(pic, controller, collection_canister_id, &init_args);
    assert!(init_response.is_ok(), "init_private_content_upload failed");

    let store_response = store_private_content_chunk(
        pic,
        controller,
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
        controller,
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
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries: entries.clone(),
                }),
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
        init_private_content_upload(pic, controller, collection_canister_id, &init_args);
    assert!(init_response.is_ok(), "init_private_content_upload failed");

    let store_response = store_private_content_chunk(
        pic,
        controller,
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
        controller,
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
                private_content: Some(NftPrivateRecordMint {
                    default_readers: HashMap::new(),
                    entries: entries.clone(),
                }),
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
    let storage_path_new = "/private/overwritten.bin".to_string();

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

    // FIXME: should not be FInalized in the storage cnaister (?)
    assert_eq!(
        get_upload_status(
            pic,
            controller,
            collection_canister_id,
            &storage_path.clone()
        )
        .unwrap(),
        UploadState::Finalized
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

    // FIXME: here it allows for owner1 to set new readers
    // // 4. Update Readers (trigger PendingReencryption)
    // let mut new_readers = HashMap::new();
    // new_readers.insert(
    //     nft_owner2,
    //     ReaderInfo {
    //         rights: AccessRights::Read,
    //         alias: None,
    //     },
    // );

    // // Prepare for reader addition
    // set_readers(
    //     pic,
    //     nft_owner1,
    //     collection_canister_id,
    //     &set_readers::Args {
    //         token_id: token_id.clone(),
    //         entry_name: entry_name.clone(),
    //         readers: new_readers.clone(),
    //     },
    // )
    // .unwrap();

    // // Assert status: PendingReencryption
    // assert_eq!(
    //     __get_private_entry_test(
    //         pic,
    //         controller,
    //         collection_canister_id,
    //         &Args {
    //             token_id: token_id.clone(),
    //             entry_name: entry_name.clone()
    //         }
    //     )
    //     .unwrap()
    //     .status,
    //     PrivateContentStatus::PendingReencryption
    // );

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

    let storage_path_reupload = format!("{}/private/initial_state.bin", REUPLOAD_PREFIX);
    init_private_content_upload(
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
        nft_owner1,
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
        nft_owner1,
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
