use crate::client::core_nft::{finalize_upload, init_upload, store_chunk};
use crate::core_suite::setup::old_test_setup;
use crate::core_suite::setup::setup::TestEnv;
use crate::core_suite::setup::setup_core::upgrade_core_canister;
use crate::utils::{
    create_default_metadata, extract_metadata_file_path, fetch_metadata_json, mint_nft,
    random_principal, setup_http_client, upload_metadata,
};
use bity_ic_types::BuildVersion;
use candid::Nat;
use core_nft::lifecycle::Args;
use core_nft::post_upgrade::UpgradeArgs;
use core_nft::types::management::{finalize_upload, init_upload, store_chunk};
use ic_cdk::println;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[test]
fn test_upgrade_storage_canister() {
    let mut test_env: TestEnv = old_test_setup();
    println!("test_env: {:?}", test_env);
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = Path::new("./src/core_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let file_type = "image/png".to_string();
    let media_hash_id = "test.png".to_string();

    let init_upload_resp = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        }),
    );

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    while offset < buffer.len() {
        let chunk = &buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())];
        let _ = store_chunk(
            pic,
            controller,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            }),
        );

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        }),
    );

    match finalize_upload_resp {
        Ok(resp) => {
            println!("finalize_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("finalize_upload_resp error: {:?}", e);
            assert!(false);
        }
    }

    let storage_upgrade_args = Args::Upgrade(UpgradeArgs {
        version: BuildVersion::min(),
        commit_hash: "commit_hash 2".to_string(),
    });

    upgrade_core_canister(
        pic,
        collection_canister_id,
        storage_upgrade_args,
        controller,
    );

    let (rt, http_gateway) = setup_http_client(pic);
    // let metadata_file_path = extract_metadata_file_path(&metadata_url);
    // let parsed_metadata = fetch_metadata_json(
    //     &rt,
    //     &http_gateway,
    //     collection_canister_id,
    //     &metadata_file_path,
    // );
    // assert_eq!(
    //     parsed_metadata
    //         .get("description")
    //         .unwrap()
    //         .as_str()
    //         .unwrap(),
    //     "Test NFT description"
    // );
    // assert_eq!(
    //     parsed_metadata.get("name").unwrap().as_str().unwrap(),
    //     "Test NFT"
    // );
}
