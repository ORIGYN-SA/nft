use crate::client::core_nft::{
    __get_public_entry_test, append_file, cancel_upload, finalize_upload, get_upload_status,
    grant_permission, init_upload, mint, revoke_permission, store_chunk,
    update_collection_metadata, update_nft_metadata,
};
use crate::utils::create_default_icrc97_metadata;

use candid::{Encode, Nat, Principal};
use core_nft_common::types::permissions::Permission;
use icrc_ledger_types::icrc1::account::Account;

use bity_ic_storage_canister_api::types::storage::UploadState;
use core_nft_common::types::management::{
    append_file, append_file::AppendFileRequest, cancel_upload, finalize_upload, grant_permission,
    init_upload, mint, mint::MintRequest, mint::NftPublicRecordMint, revoke_permission,
    store_chunk, update_collection_metadata, update_nft_metadata,
};
use ic_cdk::println;
use sha2::{Digest, Sha256};

use crate::core_suite::setup::default_test_setup;
use crate::core_suite::setup::setup::TestEnv;
use crate::utils::{
    extract_metadata_file_path, fetch_metadata_json, free_storage_bytes, raw_get,
    setup_http_client, storage_canister_for_path, upload_bytes, upload_file, upload_metadata,
};
use bytes::Bytes;
use core_nft_common::types::sub_canister::INITIAL_CYCLES_BALANCE_TEST_MODE;
use core_nft_common::types::MAX_CONTENT_SIZE;
use http::Request;
use http_body_util::BodyExt;
use ic_agent::Agent;
use ic_http_gateway::{HttpGatewayClient, HttpGatewayRequestArgs};
use serde_json::{self, json};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

#[test]
fn test_storage_simple() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/core_suite/assets/test.png";
    let upload_path = "/test.png";

    let buffer = upload_file(
        pic,
        controller,
        collection_canister_id,
        file_path,
        upload_path,
    )
    .expect("Upload failed");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = pic.auto_progress();
    println!("url: {:?}", url);
    println!(
        "request : {:?}",
        Request::builder()
            .uri(format!("/test.png").as_str())
            .body(Bytes::new())
            .unwrap()
    );

    let agent = Agent::builder().with_url(url).build().unwrap();
    rt.block_on(async {
        agent.fetch_root_key().await.unwrap();
    });
    let http_gateway = HttpGatewayClient::builder()
        .with_agent(agent)
        .build()
        .unwrap();

    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(format!("/test.png").as_str())
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    let response_headers = response
        .canister_response
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str(), v.to_str().unwrap()))
        .collect::<Vec<(&str, &str)>>();

    assert_eq!(response.canister_response.status(), 307);
    println!("response_headers: {:?}", response_headers);
    // let expected_headers = vec![(
    //     "location",
    //     "https://uqqxf-5h777-77774-qaaaa-cai.raw.icp0.io/test.png",
    // )];

    // for (key, value) in expected_headers {
    //     assert!(response_headers.contains(&(key, value)));
    // }

    assert_eq!(
        response.canister_response.status(),
        307,
        "the collection must redirect to its storage canister"
    );

    let location = response
        .canister_response
        .headers()
        .get("location")
        .expect("redirect must carry a location")
        .to_str()
        .unwrap()
        .to_string();

    let storage_canister_id = Principal::from_str(
        location
            .split('.')
            .next()
            .unwrap()
            .replace("http://", "")
            .replace("https://", "")
            .as_str(),
    )
    .unwrap();

    // File bytes are only served on the raw domain; a certified-domain request
    // redirects there rather than serving.
    let served = raw_get(&rt, &http_gateway, storage_canister_id, &location);
    assert_eq!(
        served.canister_response.status(),
        200,
        "the storage canister must serve the file on raw"
    );

    rt.block_on(async {
        let body = served
            .canister_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        assert_eq!(body, buffer);
    });
}

#[test]
fn test_duplicate_upload() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/core_suite/assets/test.png";
    let upload_path = "/test.png";

    // First upload attempt
    upload_file(
        pic,
        controller,
        collection_canister_id,
        file_path,
        upload_path,
    )
    .expect("First upload failed");

    // Second upload attempt with the same file
    let init_upload_resp_2 = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: upload_path.to_string(),
            file_hash: Some("dummy_hash".to_string()),
            file_size: 1024,
            chunk_size: None,
        }),
    );

    match init_upload_resp_2 {
        Ok(_) => {
            println!("Duplicate upload should not be allowed");
            assert!(false);
        }
        Err(e) => {
            println!("Expected error on duplicate upload: {:?}", e);
            assert!(true);
        }
    }
}

#[test]
fn test_duplicate_chunk_upload() {
    let mut test_env: TestEnv = default_test_setup();
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
            file_hash: Some(format!("{:x}", file_hash)),
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

        // Attempt to upload the same chunk again
        let duplicate_chunk_resp = store_chunk(
            pic,
            controller,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            }),
        );

        match duplicate_chunk_resp {
            Ok(_) => {
                println!("Duplicate chunk upload should not be allowed");
                assert!(false);
            }
            Err(e) => {
                println!("Expected error on duplicate chunk upload: {:?}", e);
                assert!(true);
            }
        }

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
}

#[test]
fn test_finalize_upload_missing_chunk() {
    let mut test_env: TestEnv = default_test_setup();
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

    let _ = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: Some(format!("{:x}", file_hash)),
            file_size,
            chunk_size: None,
        }),
    );

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    // Upload all chunks except the last one
    while offset < buffer.len() - (chunk_size as usize) {
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

    // Attempt to finalize upload with a missing chunk
    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        }),
    );

    match finalize_upload_resp {
        Ok(_) => {
            println!("Finalize upload should not be allowed with missing chunk");
            assert!(false);
        }
        Err(e) => {
            println!(
                "Expected error on finalize upload with missing chunk: {:?}",
                e
            );
            assert!(true);
        }
    }
}

#[test]
fn test_cancel_upload() {
    let mut test_env: TestEnv = default_test_setup();
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

    let init_upload_resp = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test_cancel.png".to_string(),
            file_hash: Some(format!("{:x}", file_hash)),
            file_size,
            chunk_size: None,
        }),
    );

    match init_upload_resp {
        Ok(resp) => {
            println!("init_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("init_upload_resp error: {:?}", e);
        }
    }

    let cancel_upload_resp = cancel_upload(
        pic,
        controller,
        collection_canister_id,
        &(cancel_upload::Args {
            file_path: "/test_cancel.png".to_string(),
        }),
    );

    match cancel_upload_resp {
        Ok(resp) => {
            println!("cancel_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("cancel_upload_resp error: {:?}", e);
            assert!(false);
        }
    }

    // Attempt to finalize the canceled upload
    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        }),
    );

    match finalize_upload_resp {
        Ok(_) => {
            println!("Finalize upload should not be allowed for a canceled upload");
            assert!(false);
        }
        Err(e) => {
            println!(
                "Expected error on finalize upload for a canceled upload: {:?}",
                e
            );
            assert!(true);
        }
    }
}

#[test]
fn test_management_file_distribution() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/core_suite/assets/test.png";
    let mut uploaded_files = Vec::new();
    let mut canister_distribution = std::collections::HashMap::new();

    // Upload 8 files
    for i in 0..14 {
        let upload_path = format!("/test_distribution_{}.png", i);
        let result = upload_file(
            pic,
            controller,
            collection_canister_id,
            file_path,
            &upload_path,
        )
        .expect("Upload failed");

        uploaded_files.push((upload_path.clone(), result));
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = pic.auto_progress();
    let agent = Agent::builder().with_url(url).build().unwrap();
    rt.block_on(async {
        agent.fetch_root_key().await.unwrap();
    });
    let http_gateway = HttpGatewayClient::builder()
        .with_agent(agent)
        .build()
        .unwrap();

    // Verify distribution of files across canisters
    for (upload_path, _original_buffer) in &uploaded_files {
        let response = rt.block_on(async {
            http_gateway
                .request(HttpGatewayRequestArgs {
                    canister_id: collection_canister_id.clone(),
                    canister_request: Request::builder()
                        .uri(upload_path.as_str())
                        .body(Bytes::new())
                        .unwrap(),
                })
                .send()
                .await
        });

        let location = response
            .canister_response
            .headers()
            .get("location")
            .unwrap_or_else(|| {
                panic!(
                    "{} must redirect to the storage canister holding it",
                    upload_path
                )
            });
        let location_str = location.to_str().unwrap();
        let canister_id = Principal::from_str(
            location_str
                .split('.')
                .next()
                .unwrap()
                .replace("http://", "")
                .replace("https://", "")
                .as_str(),
        )
        .unwrap();

        assert_ne!(
            canister_id, collection_canister_id,
            "{} must live in a storage sub-canister, not in the collection",
            upload_path
        );

        canister_distribution
            .entry(canister_id.to_string())
            .or_insert_with(Vec::new)
            .push(upload_path.clone());
    }

    // Every upload has to be accounted for: a file that resolved nowhere would
    // otherwise vanish from the distribution silently.
    let placed: usize = canister_distribution.values().map(|f| f.len()).sum();
    assert_eq!(
        placed,
        uploaded_files.len(),
        "every uploaded file must resolve to a storage canister"
    );

    // All 14 files share one canister. 14 * 6.2 MB is about 87 MB, comfortably
    // inside a test-mode storage canister's 500 MiB ceiling, so the collection
    // has no reason to spawn a second one.
    //
    // This is the assertion that catches the `Err(_) => continue` hazard in
    // `StorageSubCanisterManager::init_upload`: that arm reads every rejection
    // from a sub-canister as "this one is full", so a fleet that rejects uploads
    // for any other reason grows one canister per upload instead of failing.
    // Splitting on a genuinely full canister is covered by
    // `test_storage_threshold_splitting`, which is the one test that pays the
    // cost of actually filling one.
    assert_eq!(
        canister_distribution.len(),
        1,
        "14 small files must all fit one storage canister, but they landed on {}: {:?}",
        canister_distribution.len(),
        canister_distribution
    );
}

#[test]
fn test_management_upload_resilience() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/core_suite/assets/test.png";
    let too_big = "./src/storage_suite/assets/sbl_hero_1080_1.mp4";

    // First upload to fill up first canister partially
    let first_upload_path = "/test_resilience_1.png";
    let _ = upload_file(
        pic,
        controller,
        collection_canister_id,
        file_path,
        first_upload_path,
    )
    .expect("First upload failed");

    // Try uploading with invalid data to simulate failure
    let second_upload_path = "/test_resilience_2.png";
    let result = upload_file(
        pic,
        controller,
        collection_canister_id,
        too_big,
        second_upload_path,
    );

    println!("result: {:?}", result);

    // System should remain stable after failed upload
    let third_upload_path = "/test_resilience_3.png";
    let _ = upload_file(
        pic,
        controller,
        collection_canister_id,
        file_path,
        third_upload_path,
    )
    .expect("Third upload failed");

    // Verify files are still accessible and properly distributed
    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = pic.auto_progress();
    let agent = Agent::builder().with_url(url).build().unwrap();
    rt.block_on(async {
        agent.fetch_root_key().await.unwrap();
    });
    let http_gateway = HttpGatewayClient::builder()
        .with_agent(agent)
        .build()
        .unwrap();

    let mut unique_canisters = std::collections::HashSet::new();

    // Check first file
    let response1 = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(first_upload_path)
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    assert_eq!(response1.canister_response.status(), 307);
    if let Some(location) = response1.canister_response.headers().get("location") {
        let location_str = location.to_str().unwrap();
        let canister_id = Principal::from_str(
            location_str
                .split('.')
                .next()
                .unwrap()
                .replace("http://", "")
                .as_str(),
        )
        .unwrap();
        unique_canisters.insert(canister_id.to_string());
    }

    // Check third file
    let response3 = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(third_upload_path)
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    assert_eq!(response3.canister_response.status(), 307);
    if let Some(location) = response3.canister_response.headers().get("location") {
        let location_str = location.to_str().unwrap();
        let canister_id = Principal::from_str(
            location_str
                .split('.')
                .next()
                .unwrap()
                .replace("http://", "")
                .as_str(),
        )
        .unwrap();
        unique_canisters.insert(canister_id.to_string());
    }

    // Verify system stability is maintained
    assert!(
        unique_canisters.len() <= 2,
        "System should not create more than 2 canisters even after failed uploads"
    );
}

#[test]
fn test_management_cycles() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/core_suite/assets/test.png";
    let mut canister_cycles = std::collections::HashMap::new();

    // Record initial cycles of the collection canister
    let initial_collection_cycles = pic.cycle_balance(collection_canister_id);
    println!(
        "Initial collection canister cycles: {}",
        initial_collection_cycles
    );

    // Upload first file - should create first storage canister
    let first_upload_path = "/test_cycles_1.png";
    let _ = upload_file(
        pic,
        controller,
        collection_canister_id,
        file_path,
        first_upload_path,
    )
    .expect("First upload failed");

    // Get the first storage canister ID and record its cycles
    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = pic.auto_progress();
    let agent = Agent::builder().with_url(url).build().unwrap();
    rt.block_on(async {
        agent.fetch_root_key().await.unwrap();
    });
    let http_gateway = HttpGatewayClient::builder()
        .with_agent(agent)
        .build()
        .unwrap();

    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(first_upload_path)
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    pic.advance_time(Duration::from_secs(120));
    pic.tick();
    pic.advance_time(Duration::from_secs(120));
    pic.tick();

    if let Some(location) = response.canister_response.headers().get("location") {
        let location_str = location.to_str().unwrap();
        println!("location_str: {:?}", location_str);
        let first_storage_canister = Principal::from_str(
            location_str
                .split('.')
                .next()
                .unwrap()
                .replace("http://", "")
                .as_str(),
        )
        .unwrap();

        let first_storage_cycles = pic.cycle_balance(first_storage_canister);
        canister_cycles.insert(first_storage_canister.to_string(), first_storage_cycles);
        println!("First storage canister cycles: {}", first_storage_cycles);
    }

    // Upload more files until we create a second canister
    for i in 2..5 {
        let upload_path = format!("/test_cycles_{}.png", i);
        let _ = upload_file(
            pic,
            controller,
            collection_canister_id,
            file_path,
            &upload_path,
        )
        .expect("Upload failed");

        // Check the response to detect new canister creation
        let response = rt.block_on(async {
            http_gateway
                .request(HttpGatewayRequestArgs {
                    canister_id: collection_canister_id.clone(),
                    canister_request: Request::builder()
                        .uri(upload_path.as_str())
                        .body(Bytes::new())
                        .unwrap(),
                })
                .send()
                .await
        });

        if let Some(location) = response.canister_response.headers().get("location") {
            let location_str = location.to_str().unwrap();
            let storage_canister = Principal::from_str(
                location_str
                    .split('.')
                    .next()
                    .unwrap()
                    .replace("http://", "")
                    .as_str(),
            )
            .unwrap();

            if !canister_cycles.contains_key(&storage_canister.to_string()) {
                let storage_cycles = pic.cycle_balance(storage_canister);
                canister_cycles.insert(storage_canister.to_string(), storage_cycles);
                println!("New storage canister cycles: {}", storage_cycles);
            }
        }
    }

    // Verify cycles management
    let final_collection_cycles = pic.cycle_balance(collection_canister_id);
    println!(
        "Final collection canister cycles: {}",
        final_collection_cycles
    );

    // Verify cycles were spent from collection canister
    assert!(
        final_collection_cycles < initial_collection_cycles,
        "Collection canister should have spent cycles"
    );

    // Verify each storage canister is left holding its test-mode endowment.
    //
    // The endowment is `INITIAL_CYCLES_BALANCE_TEST_MODE` (0.5 T). canfund's
    // test-mode floor is 0.3 T with a 0.5 T refill, both inline in
    // `default_funding_config` in core_nft_common, and the point of those numbers
    // is that a healthy storage canister sits above the floor and is therefore
    // never topped up out of the collection's balance. So the assertion is that
    // the canister is still comfortably above the floor, not that it holds some
    // absolute amount: a balance below it would mean canfund is about to bill the
    // collection, and a balance above the endowment would mean it already has.
    const CANFUND_TEST_MODE_FLOOR: u128 = 300_000_000_000;
    for (canister_id, cycles) in &canister_cycles {
        assert!(
            *cycles > CANFUND_TEST_MODE_FLOOR,
            "Storage canister {} fell under the canfund top-up floor: {} <= {}",
            canister_id,
            cycles,
            CANFUND_TEST_MODE_FLOOR
        );
        assert!(
            *cycles <= INITIAL_CYCLES_BALANCE_TEST_MODE,
            "Storage canister {} was topped up past its endowment: {} > {}",
            canister_id,
            cycles,
            INITIAL_CYCLES_BALANCE_TEST_MODE
        );
    }

    // Print final cycle distribution
    println!("Final cycle distribution:");
    for (canister_id, cycles) in &canister_cycles {
        println!("Canister {}: {} cycles", canister_id, cycles);
    }
}

#[test]
#[should_panic]
fn test_update_nft_metadata_unauthorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let metadata_json = json!({
        "description": "Unauthorized test metadata",
        "name": "unauthorized_test",
        "attributes": [
            {
                "trait_type": "unauthorized",
                "value": "should_fail"
            }
        ]
    });

    let metadata_url =
        upload_metadata(pic, controller, collection_canister_id, metadata_json).unwrap();

    let unauthorized_principal = nft_owner1;
    let _ = update_nft_metadata(
        pic,
        unauthorized_principal,
        collection_canister_id,
        &(update_nft_metadata::Args {
            token_id: Nat::from(0u64),
            metadata: create_default_icrc97_metadata(metadata_url),
        }),
    );
}

#[test]
#[should_panic]
fn test_init_upload_unauthorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let unauthorized_principal = nft_owner1;
    let _ = init_upload(
        pic,
        unauthorized_principal,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: Some("dummy_hash".to_string()),
            file_size: 1024,
            chunk_size: None,
        }),
    );
}

#[test]
#[should_panic]
fn test_store_chunk_unauthorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let unauthorized_principal = nft_owner1;
    let _ = store_chunk(
        pic,
        unauthorized_principal,
        collection_canister_id,
        &(store_chunk::Args {
            file_path: "/test.png".to_string(),
            chunk_id: Nat::from(0u64),
            chunk_data: vec![0; 1024],
        }),
    );
}

#[test]
#[should_panic]
fn test_finalize_upload_unauthorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let unauthorized_principal = nft_owner1;
    let _ = finalize_upload(
        pic,
        unauthorized_principal,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        }),
    );
}

#[test]
#[should_panic]
fn test_cancel_upload_unauthorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let unauthorized_principal = nft_owner1;
    let _ = cancel_upload(
        pic,
        unauthorized_principal,
        collection_canister_id,
        &(cancel_upload::Args {
            file_path: "/test.png".to_string(),
        }),
    );
}

#[test]
#[should_panic]
fn test_mint_unauthorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let metadata_json = json!({
        "description": "Unauthorized mint test",
        "name": "unauthorized_mint",
        "attributes": [
            {
                "trait_type": "unauthorized_mint",
                "value": "should_fail"
            }
        ]
    });

    let metadata_url =
        upload_metadata(pic, controller, collection_canister_id, metadata_json).unwrap();

    let unauthorized_principal = nft_owner1;
    let result = mint(
        pic,
        unauthorized_principal,
        collection_canister_id,
        &(mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: create_default_icrc97_metadata(metadata_url),
                private_content: None,
                public_content: None,
            }],
        }),
    );
    assert!(false, "mint should panic");
}

#[test]
fn test_mint_authorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    );
    assert!(result.is_ok(), "Should succeed with authorized principal");

    let metadata_json = json!({
        "description": "Test NFT for authorized mint",
        "name": "test",
        "attributes": [
            {
                "trait_type": "authorized_test",
                "value": "success"
            }
        ]
    });

    let metadata_url =
        upload_metadata(pic, controller, collection_canister_id, metadata_json).unwrap();

    let result = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &(mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: create_default_icrc97_metadata(metadata_url.clone()),
                private_content: None,
                public_content: None,
            }],
        }),
    );
    assert!(result.is_ok(), "Should succeed with authorized principal");

    let (rt, http_gateway) = setup_http_client(pic);
    let metadata_file_path = extract_metadata_file_path(&metadata_url);
    let parsed_metadata = fetch_metadata_json(
        &rt,
        &http_gateway,
        collection_canister_id,
        &metadata_file_path,
        true,
    );

    assert_eq!(
        parsed_metadata.get("name").unwrap().as_str().unwrap(),
        "test"
    );
    assert_eq!(
        parsed_metadata
            .get("attributes")
            .unwrap()
            .get(0)
            .unwrap()
            .get("trait_type")
            .unwrap()
            .as_str()
            .unwrap(),
        "authorized_test"
    );
}

#[test]
#[should_panic]
fn test_add_then_remove_minting_authorities_unauthorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    );
    assert!(result.is_ok(), "Should succeed with authorized principal");

    let result = revoke_permission(
        pic,
        controller,
        collection_canister_id,
        &(revoke_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    );
    assert!(result.is_ok(), "Should succeed with authorized principal");

    let metadata_json = json!({
        "description": "Removed minting authority test",
        "name": "removed_authority_test",
        "attributes": [
            {
                "trait_type": "removed_authority",
                "value": "should_fail"
            }
        ]
    });

    let metadata_url =
        upload_metadata(pic, controller, collection_canister_id, metadata_json).unwrap();

    let result = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &(mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: create_default_icrc97_metadata(metadata_url),
                private_content: None,
                public_content: None,
            }],
        }),
    );
    assert!(false, "should panic");
}

#[test]
fn test_mint_with_metadata() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let metadata_json = json!({
        "description": "test",
        "name": "test",
        "attributes": [
            {
                "trait_type": "test1",
                "value": "test1"
            },
            {
                "trait_type": "test2",
                "value": "test2"
            },
            {
                "display_type": "number",
                "trait_type": "test4",
                "value": 2
            }
        ]
    });

    let metadata_url =
        upload_metadata(pic, controller, collection_canister_id, metadata_json).unwrap();

    let result = mint(
        pic,
        controller,
        collection_canister_id,
        &(mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: create_default_icrc97_metadata(metadata_url.clone()),
                private_content: None,
                public_content: None,
            }],
        }),
    );
    assert!(result.is_ok(), "Should succeed with authorized principal");

    let (rt, http_gateway) = setup_http_client(pic);
    let metadata_file_path = extract_metadata_file_path(&metadata_url);

    let parsed_metadata = fetch_metadata_json(
        &rt,
        &http_gateway,
        collection_canister_id,
        &metadata_file_path,
        true,
    );

    println!("parsed_metadata: {:?}", parsed_metadata);

    assert!(
        parsed_metadata
            .get("attributes")
            .unwrap()
            .get(0)
            .unwrap()
            .get("trait_type")
            .unwrap()
            .as_str()
            .unwrap()
            .eq("test1"),
        "The metadata 'test1' should be present"
    );
    assert_eq!(
        parsed_metadata
            .get("attributes")
            .unwrap()
            .get(0)
            .unwrap()
            .get("value")
            .unwrap()
            .as_str()
            .unwrap(),
        "test1",
        "The value of 'test1' should be 'test1'"
    );

    assert!(
        parsed_metadata
            .get("attributes")
            .unwrap()
            .get(1)
            .unwrap()
            .get("trait_type")
            .unwrap()
            .as_str()
            .unwrap()
            .eq("test2"),
        "The metadata 'test2' should be present"
    );
    assert_eq!(
        parsed_metadata
            .get("attributes")
            .unwrap()
            .get(1)
            .unwrap()
            .get("value")
            .unwrap()
            .as_str()
            .unwrap(),
        "test2",
        "The value of 'test2' should be 'test2'"
    );

    println!("Verification of the JSON file metadata successful!");
}

#[test]
fn test_get_upload_status() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/core_suite/assets/test.png";
    let upload_path = "/test_status.png";
    let upload_path2 = "/test_status2.png";

    let status_before = get_upload_status(
        pic,
        controller,
        collection_canister_id,
        &upload_path.to_string(),
    );
    assert!(
        matches!(
            status_before,
            Err(core_nft_common::types::management::get_upload_status::GetUploadStatusError::UploadNotFound)
        ),
        "Should return error for non-existent upload"
    );

    let init_upload_resp = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: upload_path2.to_string(),
            file_hash: Some("dummy_hash".to_string()),
            file_size: 1024,
            chunk_size: None,
        }),
    );
    assert!(init_upload_resp.is_ok(), "Init upload should succeed");

    let status_after_init = get_upload_status(
        pic,
        controller,
        collection_canister_id,
        &upload_path2.to_string(),
    );
    assert!(
        matches!(status_after_init, Ok(UploadState::Init)),
        "Should return Init state"
    );

    let _ = upload_file(
        pic,
        controller,
        collection_canister_id,
        file_path,
        upload_path,
    )
    .expect("Upload failed");

    let status_after_upload = get_upload_status(
        pic,
        controller,
        collection_canister_id,
        &upload_path.to_string(),
    );
    assert!(
        matches!(status_after_upload, Ok(UploadState::Finalized)),
        "Should return Finalized state"
    );
}

#[test]
fn test_get_all_uploads() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/core_suite/assets/test.png";
    let mut upload_paths = Vec::new();

    for i in 0..3 {
        let upload_path = format!("/test_all_uploads_{}.png", i);
        let _ = upload_file(
            pic,
            controller,
            collection_canister_id,
            file_path,
            &upload_path,
        )
        .expect("Upload failed");
        upload_paths.push(upload_path);
    }

    let all_uploads: core_nft_common::types::management::get_all_uploads::Response =
        crate::client::pocket::unwrap_response(pic.query_call(
            collection_canister_id,
            controller,
            "get_all_uploads",
            Encode!(&(), &()).unwrap(),
        ));

    assert_eq!(all_uploads.unwrap().len(), 3, "Should return all 3 uploads");

    // Test pagination
    let first_page: core_nft_common::types::management::get_all_uploads::Response =
        crate::client::pocket::unwrap_response(pic.query_call(
            collection_canister_id,
            controller,
            "get_all_uploads",
            Encode!(&Some(Nat::from(0u64)), &Some(Nat::from(2u64))).unwrap(),
        ));

    assert_eq!(
        first_page.unwrap().len(),
        2,
        "Should return 2 uploads for first page"
    );

    let second_page: core_nft_common::types::management::get_all_uploads::Response =
        crate::client::pocket::unwrap_response(pic.query_call(
            collection_canister_id,
            controller,
            "get_all_uploads",
            Encode!(&Some(Nat::from(2u64)), &Some(Nat::from(2u64))).unwrap(),
        ));

    assert_eq!(
        second_page.unwrap().len(),
        1,
        "Should return 1 upload for second page"
    );
}

#[test]
fn test_update_collection_metadata() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    // Test updating collection metadata
    let result = update_collection_metadata(
        pic,
        controller,
        collection_canister_id,
        &(update_collection_metadata::Args {
            description: Some("Test Description".to_string()),
            symbol: Some("TEST".to_string()),
            name: Some("Test Collection".to_string()),
            logo: Some("https://google.com/test.png".to_string()),
            supply_cap: Some(Nat::from(1000u64)),
            max_query_batch_size: Some(Nat::from(100u64)),
            max_update_batch_size: Some(Nat::from(50u64)),
            max_take_value: Some(Nat::from(200u64)),
            default_take_value: Some(Nat::from(20u64)),
            max_memo_size: Some(Nat::from(32u64)),
            atomic_batch_transfers: Some(true),
            tx_window: Some(Nat::from(3600u64)),
            permitted_drift: Some(Nat::from(60u64)),
            max_canister_storage_threshold: Some(Nat::from(0u64)),
            collection_metadata: Some(HashMap::new()),
        }),
    );
    assert!(
        result.is_ok(),
        "Should update collection metadata successfully"
    );
}

#[test]
fn test_update_collection_metadata_custom() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    use crate::client::core_nft::icrc7_collection_metadata;
    use core_nft_common::types::value_custom::CustomValue;
    use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;

    let mut custom_metadata = HashMap::new();
    custom_metadata.insert(
        "custom:field1".to_string(),
        CustomValue(Value::Text("value1".to_string())),
    );
    custom_metadata.insert(
        "custom:field2".to_string(),
        CustomValue(Value::Nat(Nat::from(42u64))),
    );

    // Test updating collection metadata with custom fields
    let result = update_collection_metadata(
        pic,
        controller,
        collection_canister_id,
        &(update_collection_metadata::Args {
            description: None,
            symbol: None,
            name: None,
            logo: None,
            supply_cap: None,
            max_query_batch_size: None,
            max_update_batch_size: None,
            max_take_value: None,
            default_take_value: None,
            max_memo_size: None,
            atomic_batch_transfers: None,
            tx_window: None,
            permitted_drift: None,
            max_canister_storage_threshold: None,
            collection_metadata: Some(custom_metadata),
        }),
    );
    assert!(
        result.is_ok(),
        "Should update collection metadata successfully"
    );

    // Retrieve and verify custom metadata
    let metadata = icrc7_collection_metadata(pic, controller, collection_canister_id, &());

    assert!(metadata
        .iter()
        .any(|(key, value)| key == "custom:field1"
            && matches!(value, Value::Text(s) if s == "value1")));
    assert!(metadata.iter().any(|(key, value)| key == "custom:field2"
        && matches!(value, Value::Nat(n) if n == &Nat::from(42u64))));

    // Now test validation (rejecting protected keys in custom metadata)
    let mut invalid_metadata = HashMap::new();
    invalid_metadata.insert(
        "icrc7:symbol".to_string(),
        CustomValue(Value::Text("BAD".to_string())),
    );

    let invalid_result = update_collection_metadata(
        pic,
        controller,
        collection_canister_id,
        &(update_collection_metadata::Args {
            description: None,
            symbol: None,
            name: None,
            logo: None,
            supply_cap: None,
            max_query_batch_size: None,
            max_update_batch_size: None,
            max_take_value: None,
            default_take_value: None,
            max_memo_size: None,
            atomic_batch_transfers: None,
            tx_window: None,
            permitted_drift: None,
            max_canister_storage_threshold: None,
            collection_metadata: Some(invalid_metadata),
        }),
    );

    assert!(
        matches!(
            invalid_result,
            Err(core_nft_common::types::management::update_collection_metadata::UpdateCollectionMetadataError::InvalidMetadataKey(_))
        ),
        "Should fail when trying to update with a protected key"
    );
}

#[test]
#[should_panic]
fn test_update_collection_metadata_unauthorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    // Test unauthorized update
    let unauthorized_result = update_collection_metadata(
        pic,
        nft_owner1,
        collection_canister_id,
        &(update_collection_metadata::Args {
            description: Some("Unauthorized Update".to_string()),
            symbol: None,
            name: None,
            logo: None,
            supply_cap: None,
            max_query_batch_size: None,
            max_update_batch_size: None,
            max_take_value: None,
            default_take_value: None,
            max_memo_size: None,
            atomic_batch_transfers: None,
            tx_window: None,
            permitted_drift: None,
            max_canister_storage_threshold: None,
            collection_metadata: None,
        }),
    );
    assert!(
        matches!(
            unauthorized_result,
            Err(core_nft_common::types::management::update_collection_metadata::UpdateCollectionMetadataError::ConcurrentManagementCall)
        ),
        "Should fail for unauthorized principal"
    );
}

#[test]
fn test_permissions_add_and_remove_one_by_one() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let test_principal = nft_owner1;

    let metadata_json = json!({
        "description": "Test before minting permission",
        "name": "test_before_minting",
        "attributes": [{"trait_type": "test", "value": "before"}]
    });
    let metadata_url =
        upload_metadata(pic, controller, collection_canister_id, metadata_json).unwrap();

    let grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: test_principal,
            permission: Permission::Minting,
        }),
    );
    assert!(
        grant_result.is_ok(),
        "Should grant minting permission successfully"
    );

    // Verify minting works after permission
    let metadata_json = json!({
        "description": "Test after minting permission",
        "name": "test_after_minting",
        "attributes": [{"trait_type": "test", "value": "after"}]
    });
    let metadata_url =
        upload_metadata(pic, controller, collection_canister_id, metadata_json).unwrap();

    let mint_result_after = mint(
        pic,
        test_principal,
        collection_canister_id,
        &(mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: test_principal,
                    subaccount: None,
                },
                memo: None,
                metadata: create_default_icrc97_metadata(metadata_url),
                private_content: None,
                public_content: None,
            }],
        }),
    );
    assert!(
        mint_result_after.is_ok(),
        "Minting should work after permission is granted"
    );
    let token_id = mint_result_after.unwrap();

    // Revoke minting permission
    let revoke_result = revoke_permission(
        pic,
        controller,
        collection_canister_id,
        &(revoke_permission::Args {
            principal: test_principal,
            permission: Permission::Minting,
        }),
    );
    assert!(
        revoke_result.is_ok(),
        "Should revoke minting permission successfully"
    );

    let grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: test_principal,
            permission: Permission::UpdateMetadata,
        }),
    );
    assert!(
        grant_result.is_ok(),
        "Should grant UpdateMetadata permission successfully"
    );

    // Verify metadata update works after permission
    let update_metadata_json = json!({
        "description": "Authorized update attempt",
        "name": "authorized_update",
        "attributes": [{"trait_type": "authorized", "value": "should_work"}]
    });
    let update_metadata_url = upload_metadata(
        pic,
        controller,
        collection_canister_id,
        update_metadata_json,
    )
    .unwrap();

    let update_result_after = update_nft_metadata(
        pic,
        test_principal,
        collection_canister_id,
        &(update_nft_metadata::Args {
            token_id: token_id.clone(),
            metadata: create_default_icrc97_metadata(update_metadata_url),
        }),
    );
    assert!(
        update_result_after.is_ok(),
        "Metadata update should work after permission is granted"
    );

    // Revoke UpdateMetadata permission
    let revoke_result = revoke_permission(
        pic,
        controller,
        collection_canister_id,
        &(revoke_permission::Args {
            principal: test_principal,
            permission: Permission::UpdateMetadata,
        }),
    );
    assert!(
        revoke_result.is_ok(),
        "Should revoke UpdateMetadata permission successfully"
    );

    let grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: test_principal,
            permission: Permission::UpdateUploads,
        }),
    );
    assert!(
        grant_result.is_ok(),
        "Should grant UpdateUploads permission successfully"
    );

    let init_result_after = init_upload(
        pic,
        test_principal,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test_permissions.png".to_string(),
            file_hash: Some("dummy_hash".to_string()),
            file_size: 1024,
            chunk_size: None,
        }),
    );
    assert!(
        init_result_after.is_ok(),
        "Upload should work after permission is granted"
    );

    let revoke_result = revoke_permission(
        pic,
        controller,
        collection_canister_id,
        &(revoke_permission::Args {
            principal: test_principal,
            permission: Permission::UpdateUploads,
        }),
    );
    assert!(
        revoke_result.is_ok(),
        "Should revoke UpdateUploads permission successfully"
    );

    let grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: test_principal,
            permission: Permission::ReadUploads,
        }),
    );
    assert!(
        grant_result.is_ok(),
        "Should grant ReadUploads permission successfully"
    );

    let status_result_after = get_upload_status(
        pic,
        test_principal,
        collection_canister_id,
        &"/test_permissions.png".to_string(),
    );
    assert!(
        status_result_after.is_ok(),
        "Get upload status should work after permission is granted"
    );

    let revoke_result = revoke_permission(
        pic,
        controller,
        collection_canister_id,
        &(revoke_permission::Args {
            principal: test_principal,
            permission: Permission::ReadUploads,
        }),
    );
    assert!(
        revoke_result.is_ok(),
        "Should revoke ReadUploads permission successfully"
    );

    let grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: test_principal,
            permission: Permission::UpdateCollectionMetadata,
        }),
    );
    assert!(
        grant_result.is_ok(),
        "Should grant UpdateCollectionMetadata permission successfully"
    );

    let collection_update_result_after = update_collection_metadata(
        pic,
        test_principal,
        collection_canister_id,
        &(update_collection_metadata::Args {
            description: Some("Authorized collection update".to_string()),
            symbol: None,
            name: None,
            logo: None,
            supply_cap: None,
            max_query_batch_size: None,
            max_update_batch_size: None,
            max_take_value: None,
            default_take_value: None,
            max_memo_size: None,
            atomic_batch_transfers: None,
            tx_window: None,
            permitted_drift: None,
            max_canister_storage_threshold: None,
            collection_metadata: None,
        }),
    );
    assert!(
        collection_update_result_after.is_ok(),
        "Collection metadata update should work after permission is granted"
    );

    let revoke_result = revoke_permission(
        pic,
        controller,
        collection_canister_id,
        &(revoke_permission::Args {
            principal: test_principal,
            permission: Permission::UpdateCollectionMetadata,
        }),
    );
    assert!(
        revoke_result.is_ok(),
        "Should revoke UpdateCollectionMetadata permission successfully"
    );

    let grant_result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: test_principal,
            permission: Permission::ManageAuthorities,
        }),
    );
    assert!(
        grant_result.is_ok(),
        "Should grant ManageAuthorities permission successfully"
    );

    let permission_result_after = grant_permission(
        pic,
        test_principal,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner2,
            permission: Permission::Minting,
        }),
    );
    assert!(
        permission_result_after.is_ok(),
        "Permission management should work after permission is granted"
    );

    let revoke_result = revoke_permission(
        pic,
        controller,
        collection_canister_id,
        &(revoke_permission::Args {
            principal: test_principal,
            permission: Permission::ManageAuthorities,
        }),
    );
    assert!(
        revoke_result.is_ok(),
        "Should revoke ManageAuthorities permission successfully"
    );
}

#[test]
fn test_append_file_authorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    // 1. Grant minting and update metadata permission
    let result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::Minting,
        }),
    );
    assert!(result.is_ok(), "Should succeed with authorized principal");

    let result = grant_permission(
        pic,
        controller,
        collection_canister_id,
        &(grant_permission::Args {
            principal: nft_owner1,
            permission: Permission::UpdateMetadata,
        }),
    );
    assert!(result.is_ok(), "Should succeed with authorized principal");

    // 2. Mint an NFT
    let mint_result = mint(
        pic,
        nft_owner1,
        collection_canister_id,
        &(mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: None,
                public_content: None,
            }],
        }),
    );
    assert!(mint_result.is_ok());
    let token_id = mint_result.unwrap();

    // 3. Upload a public file
    let file_path = "./src/core_suite/assets/test.png";
    let upload_path = "/test_append.png";
    upload_file(
        pic,
        controller,
        collection_canister_id,
        file_path,
        upload_path,
    )
    .expect("Upload failed");

    // 4. Append the file using append_file
    let mut entries = HashMap::new();
    entries.insert("test_entry".to_string(), upload_path.to_string());

    let append_result = append_file(
        pic,
        nft_owner1,
        collection_canister_id,
        &(append_file::Args {
            append_file_requests: vec![AppendFileRequest {
                token_id: token_id.clone(),
                public_content: Some(
                    core_nft_common::types::management::append_file::NftPublicRecordAppend {
                        entries,
                    },
                ),
            }],
        }),
    );
    assert!(append_result.is_ok(), "Append file should succeed");

    // 5. Query the public entry using the test query to verify it was appended
    let get_public_result = __get_public_entry_test(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::queries::public_content::__get_public_entry_test::Args {
            token_id,
            entry_name: "test_entry".to_string(),
        },
    );

    assert!(
        get_public_result.is_ok(),
        "Query public entry should succeed"
    );
    let entry = get_public_result.unwrap();
    assert_eq!(entry.hash, upload_path.to_string());
}

#[test]
fn test_mint_with_public_content() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    // 1. Upload a public file first (mint requires it finalized).
    let upload_path = "/test_mint_public.png";
    upload_file(
        pic,
        controller,
        collection_canister_id,
        "./src/core_suite/assets/test.png",
        upload_path,
    )
    .expect("Upload failed");

    // 2. Mint a token that attaches the uploaded file as public content.
    let mut entries = HashMap::new();
    entries.insert("main".to_string(), upload_path.to_string());

    let mint_result = mint(
        pic,
        controller,
        collection_canister_id,
        &(mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: controller,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: None,
                public_content: Some(NftPublicRecordMint { entries }),
            }],
        }),
    );
    assert!(
        mint_result.is_ok(),
        "mint with public content failed: {mint_result:?}"
    );
    let token_id = mint_result.unwrap();

    // 3. The file is attached to the freshly minted token.
    let get_public_result = __get_public_entry_test(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::queries::public_content::__get_public_entry_test::Args {
            token_id,
            entry_name: "main".to_string(),
        },
    );
    assert!(
        get_public_result.is_ok(),
        "public entry should exist after mint: {get_public_result:?}"
    );
    assert_eq!(get_public_result.unwrap().hash, upload_path.to_string());
}

#[test]
fn test_mint_with_missing_public_content_is_rejected() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    // Reference a file that was never uploaded -> mint must fail cleanly.
    let mut entries = HashMap::new();
    entries.insert("main".to_string(), "/never_uploaded.png".to_string());

    let mint_result = mint(
        pic,
        controller,
        collection_canister_id,
        &(mint::Args {
            mint_requests: vec![MintRequest {
                token_owner: Account {
                    owner: controller,
                    subaccount: None,
                },
                memo: None,
                metadata: vec![],
                private_content: None,
                public_content: Some(NftPublicRecordMint { entries }),
            }],
        }),
    );
    assert!(
        mint_result.is_err(),
        "mint referencing an unknown public file must be rejected"
    );
}

#[test]
fn test_storage_size_checks() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    let file_path = "./src/core_suite/assets/test.png";
    let upload_path = "/test_size_check.png";

    // 1. Upload a file
    let buffer = upload_file(
        pic,
        controller,
        collection_canister_id,
        file_path,
        upload_path,
    )
    .expect("Upload failed");

    let file_size = buffer.len() as u64;

    // 2. Fetch storage canister ID via HTTP request redirect
    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = pic.auto_progress();
    let agent = Agent::builder().with_url(url).build().unwrap();
    rt.block_on(async {
        agent.fetch_root_key().await.unwrap();
    });
    let http_gateway = HttpGatewayClient::builder()
        .with_agent(agent)
        .build()
        .unwrap();

    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(upload_path)
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    let location = response
        .canister_response
        .headers()
        .get("location")
        .expect("Location header not found");
    let location_str = location.to_str().unwrap();
    let storage_canister_id = Principal::from_str(
        location_str
            .split('.')
            .next()
            .unwrap()
            .replace("http://", "")
            .as_str(),
    )
    .unwrap();

    // 3. Check storage canister size metrics directly
    let storage_size =
        crate::client::storage::get_storage_size(pic, controller, storage_canister_id, &());
    let stored_files_size = crate::client::storage::get_stored_files_size_bytes(
        pic,
        controller,
        storage_canister_id,
        &(),
    );

    println!("storage_size: {}", storage_size);
    println!("stored_files_size: {}", stored_files_size);

    assert_eq!(
        stored_files_size, file_size,
        "Stored files size should match the uploaded file size"
    );
    assert!(
        storage_size >= stored_files_size as u128,
        "Physical storage size must be greater than or equal to logical files size"
    );
}

#[test]
fn test_storage_limits_and_freeing_space() {
    // 1. Initialize collection canister. A storage sub-canister created in test
    //    mode is capped at `TEST_MODE_STORAGE_CEILING_BYTES` (500 MiB).
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    let upload_path_20mb = "/temp_20mb.bin";
    let upload_path_15mb = "/temp_15mb.bin";

    // 20 MB of zeros, and 15 MB that will fit in the pages it frees up.
    let data_20mb = vec![0u8; 20_000_000];
    let data_15mb = vec![0u8; 15_000_000];

    // 2. Upload the 20 MB file (well inside the ceiling, so it must succeed)
    let buffer_20mb = upload_bytes(
        pic,
        controller,
        collection_canister_id,
        &data_20mb,
        upload_path_20mb,
    );
    assert!(buffer_20mb.is_ok(), "20 MB file upload should succeed");

    // 3. Retrieve the storage canister ID via HTTP gateway redirect
    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = pic.auto_progress();
    let url_str = url.to_string();
    let agent = Agent::builder().with_url(url).build().unwrap();
    rt.block_on(async {
        agent.fetch_root_key().await.unwrap();
    });
    let http_gateway = HttpGatewayClient::builder()
        .with_agent(agent)
        .build()
        .unwrap();

    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(upload_path_20mb)
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    let location = response
        .canister_response
        .headers()
        .get("location")
        .expect("Location header not found");
    let location_str = location.to_str().unwrap();
    let storage_canister_id = Principal::from_str(
        location_str
            .split('.')
            .next()
            .unwrap()
            .replace("http://", "")
            .as_str(),
    )
    .unwrap();

    // 4. Verify initial sizes on the storage canister
    let initial_storage_size =
        crate::client::storage::get_storage_size(pic, controller, storage_canister_id, &());
    let initial_stored_files_size = crate::client::storage::get_stored_files_size_bytes(
        pic,
        controller,
        storage_canister_id,
        &(),
    );
    println!("Initial storage size: {}", initial_storage_size);
    println!("Initial stored files size: {}", initial_stored_files_size);
    assert_eq!(initial_stored_files_size, 20_000_000);

    // The ceiling is what the canister will accept, minus what it has already
    // allocated. Read it back rather than hardcoding it.
    let free_after_20mb = free_storage_bytes(pic, controller, storage_canister_id);
    assert!(
        free_after_20mb > 0,
        "a canister holding 20 MB must still have room under the 500 MiB ceiling"
    );

    // 6. Delete the 20 MB file directly from the storage canister
    let remove_resp = crate::client::storage::remove_file(
        pic,
        controller,
        storage_canister_id,
        &bity_ic_storage_canister_api::updates::remove_file::Args {
            file_path: upload_path_20mb.to_string(),
        },
    );
    println!("Remove file response: {:?}", remove_resp);

    // Verify stored files size dropped to 0
    let after_remove_stored_files_size = crate::client::storage::get_stored_files_size_bytes(
        pic,
        controller,
        storage_canister_id,
        &(),
    );
    println!(
        "Stored files size after remove: {}",
        after_remove_stored_files_size
    );
    assert_eq!(after_remove_stored_files_size, 0);

    // 7. Upload the 15 MB file (should SUCCEED now and fit inside the pre-allocated pages)
    let buffer_15mb_success = upload_bytes(
        pic,
        controller,
        collection_canister_id,
        &data_15mb,
        upload_path_15mb,
    );
    assert!(
        buffer_15mb_success.is_ok(),
        "15 MB file upload should succeed after freeing space"
    );

    // Verify final sizes and assert page reuse
    let final_storage_size =
        crate::client::storage::get_storage_size(pic, controller, storage_canister_id, &());
    let final_stored_files_size = crate::client::storage::get_stored_files_size_bytes(
        pic,
        controller,
        storage_canister_id,
        &(),
    );
    println!("Final storage size: {}", final_storage_size);
    println!("Final stored files size: {}", final_stored_files_size);
    assert_eq!(final_stored_files_size, 15_000_000);
    assert_eq!(
        final_storage_size, initial_storage_size,
        "Physical storage size should NOT grow (pages must be reused)"
    );

    // Verify that the 15 MB file was indeed uploaded to the same storage canister (reused space)
    let final_canister_id =
        resolve_storage_canister_id(url_str.as_str(), collection_canister_id, upload_path_15mb);
    assert_eq!(
        final_canister_id, storage_canister_id,
        "The reused file must be stored in the same canister"
    );

    // 8. An upload too large to store is still refused.
    //
    // The size that does it is `MAX_CONTENT_SIZE + 1`, not something over the
    // storage canister's capacity ceiling. The collection caps a single upload at
    // 100 MiB before it consults any sub-canister, and that is below the 500 MiB
    // test-mode ceiling, so a size the collection would accept always fits a fresh
    // sub-canister. This test used to use 55 MB against a 50 MiB ceiling, where
    // the sub-canister was the binding limit.
    //
    // It is kept last on purpose. The `MAX_CONTENT_SIZE` check refuses inside
    // `read_state`, so this particular rejection spawns nothing. A rejection on
    // capacity grounds does not: it runs through `create_canister` before giving
    // up, and leaves a second empty canister behind. The assertions above are
    // about which of the collection's canisters a file lands on, so keeping any
    // rejection after them removes that coupling whichever limit is doing the
    // refusing.
    let upload_path_oversize = "/temp_oversize.bin";
    let init_result = crate::client::core_nft::init_upload(
        pic,
        controller,
        collection_canister_id,
        &core_nft_common::types::management::init_upload::Args {
            file_path: upload_path_oversize.to_string(),
            file_hash: Some("dummy_hash".to_string()),
            file_size: MAX_CONTENT_SIZE + 1,
            chunk_size: None,
        },
    );
    println!("DEBUG: init_result = {:?}", init_result);
    assert!(
        init_result.is_err(),
        "an upload over the maximum content size must be rejected"
    );
}

/// A file that no longer fits the current storage canister is placed in a new one.
///
/// Splitting is driven entirely by the storage canister rejecting `init_upload`
/// once the declared size exceeds `ceiling - stable_size()`. The collection reads
/// any rejection as "that one is full", moves to the next sub-canister and spawns
/// one when it runs out. Nothing scales that ceiling down for a test:
/// `max_storage_size_for` is compiled into the storage wasm, and the collection's
/// `max_canister_storage_threshold` init arg is stored in state but never read on
/// the upload path. So proving the split means genuinely filling a canister to its
/// 500 MiB test-mode ceiling, and this is the only test that pays for it.
///
/// The fill uses 32 MiB files rather than one large one. Chunks accumulate in the
/// canister's heap until finalize assembles them into a single buffer, so one
/// 500 MiB upload would peak above a gigabyte of heap.
#[test]
fn test_storage_threshold_splitting() {
    let mut test_env = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    const FILLER_BYTES: usize = 32 * 1024 * 1024;
    let filler = vec![0u8; FILLER_BYTES];

    // The first upload spawns the canister this test fills.
    let first_path = "/split_filler_0.bin";
    upload_bytes(pic, controller, collection_canister_id, &filler, first_path)
        .expect("the first upload should succeed");

    let first_canister =
        storage_canister_for_path(pic, controller, collection_canister_id, first_path);

    // Keep loading it while there is room for another whole file. Every file that
    // still fits has to stay put: a split before the canister is full would mean
    // the collection is reading some other rejection as "full".
    let mut index = 1;
    loop {
        let free = free_storage_bytes(pic, controller, first_canister);
        println!("free bytes on {first_canister} before upload {index}: {free}");

        if free < FILLER_BYTES as u128 {
            break;
        }

        assert!(
            index < 32,
            "the canister is not filling up: after {index} files of {FILLER_BYTES} bytes it still reports {free} bytes free"
        );

        let path = format!("/split_filler_{index}.bin");
        upload_bytes(pic, controller, collection_canister_id, &filler, &path)
            .unwrap_or_else(|e| panic!("filler upload {index} should succeed: {e}"));

        assert_eq!(
            storage_canister_for_path(pic, controller, collection_canister_id, &path),
            first_canister,
            "a file that still fits must stay in the canister it was offered to"
        );

        index += 1;
    }

    assert!(
        index > 1,
        "the ceiling must leave room for more than one file, otherwise this proves nothing"
    );

    // The next file does not fit, so the collection has to place it elsewhere.
    let overflow_path = "/split_overflow.bin";
    upload_bytes(
        pic,
        controller,
        collection_canister_id,
        &filler,
        overflow_path,
    )
    .expect("an upload that overflows a canister must still succeed, in a new one");

    let overflow_canister =
        storage_canister_for_path(pic, controller, collection_canister_id, overflow_path);

    assert_ne!(
        overflow_canister, first_canister,
        "a file that no longer fits must be split into a new storage canister"
    );
}

fn resolve_storage_canister_id(
    url: &str,
    collection_canister_id: Principal,
    upload_path: &str,
) -> Principal {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let agent = Agent::builder().with_url(url.to_string()).build().unwrap();
    rt.block_on(async {
        agent.fetch_root_key().await.unwrap();
    });
    let http_gateway = HttpGatewayClient::builder()
        .with_agent(agent)
        .build()
        .unwrap();

    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(upload_path)
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    let location = response
        .canister_response
        .headers()
        .get("location")
        .expect("Location header not found");
    let location_str = location.to_str().unwrap();
    Principal::from_str(
        location_str
            .split('.')
            .next()
            .unwrap()
            .replace("http://", "")
            .as_str(),
    )
    .unwrap()
}

/// The size limit on a single upload is decided at `init_upload`, from the
/// declared size alone, and it is inclusive.
///
/// The binding limit is the collection's own `MAX_CONTENT_SIZE` (100 MiB), which
/// `init_upload` checks before it consults any storage sub-canister. It sits well
/// under a test-mode storage canister's 500 MiB capacity ceiling
/// (`TEST_MODE_STORAGE_CEILING_BYTES`), so no upload the collection will accept
/// can be too large for a fresh sub-canister to hold. That is the difference from
/// when this test was written against a 50 MiB ceiling, where the sub-canister was
/// the binding limit and a 55 MB file was refused by the whole fleet.
///
/// The ceiling still decides where a file goes once a canister has filled up, and
/// that is covered by `test_storage_threshold_splitting`, which is the one test
/// that pays for filling one.
///
/// Nothing here moves any bytes: `init_upload` rules on the declared size, so the
/// declaration is the whole behaviour under test.
#[test]
fn test_storage_edge_cases() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    // --- A file of exactly the maximum content size is accepted ---
    let exact_path = "/edge_exact_limit.bin";
    crate::client::core_nft::init_upload(
        pic,
        controller,
        collection_canister_id,
        &core_nft_common::types::management::init_upload::Args {
            file_path: exact_path.to_string(),
            file_hash: Some("dummy_hash".to_string()),
            file_size: MAX_CONTENT_SIZE,
            chunk_size: None,
        },
    )
    .expect("a file of exactly the maximum content size must be accepted");

    // Acceptance means a storage sub-canister took the reservation, so the upload
    // is now in progress rather than unknown. (The path cannot be resolved to its
    // canister over HTTP yet: the collection only serves a redirect once an upload
    // is finalized.)
    let status = get_upload_status(
        pic,
        controller,
        collection_canister_id,
        &exact_path.to_string(),
    );
    assert!(
        matches!(status, Ok(UploadState::Init) | Ok(UploadState::InProgress)),
        "the accepted upload must be registered as in progress, got {status:?}"
    );

    cancel_upload(
        pic,
        controller,
        collection_canister_id,
        &core_nft_common::types::management::cancel_upload::Args {
            file_path: exact_path.to_string(),
        },
    )
    .expect("the reserved upload should cancel cleanly");

    // --- One byte over is rejected, and rejected by the collection itself ---
    let init_result = crate::client::core_nft::init_upload(
        pic,
        controller,
        collection_canister_id,
        &core_nft_common::types::management::init_upload::Args {
            file_path: "/edge_over_limit.bin".to_string(),
            file_hash: Some("dummy_hash".to_string()),
            file_size: MAX_CONTENT_SIZE + 1,
            chunk_size: None,
        },
    );
    assert!(
        matches!(
            init_result,
            Err(core_nft_common::types::management::init_upload::InitUploadError::ContentTooLarge)
        ),
        "one byte over the maximum content size must be refused by the collection, got {init_result:?}"
    );
}

/// An upload that declares no hash must still land in a storage canister and
/// come back byte for byte.
///
/// The pass-through is the point: 0.7 made `file_hash` an `opt text`, and the
/// collection forwards `None` to the storage canister rather than computing a
/// hash of its own. Asserting only on the finalize response would not notice a
/// file that was accepted and then stored empty or truncated, so the bytes are
/// read back over the HTTP gateway.
#[test]
fn test_upload_without_declared_hash_passes_through_to_storage() {
    let mut test_env: TestEnv = default_test_setup();

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    let upload_path = "/no_declared_hash.bin".to_string();
    let data = vec![7u8; 5000];

    init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: upload_path.clone(),
            file_hash: None,
            file_size: data.len() as u64,
            chunk_size: None,
        }),
    )
    .expect("init_upload without a declared hash should succeed");

    store_chunk(
        pic,
        controller,
        collection_canister_id,
        &(store_chunk::Args {
            file_path: upload_path.clone(),
            chunk_id: Nat::from(0u64),
            chunk_data: data.clone(),
        }),
    )
    .expect("store_chunk should succeed");

    let finalize_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: upload_path.clone(),
        }),
    )
    .expect("finalize_upload should succeed when no hash was declared");

    assert!(
        finalize_resp.url.contains(&upload_path),
        "unexpected url: {}",
        finalize_resp.url
    );

    // Read it back the way a browser does: the collection answers 307 with the
    // storage canister it delegated to, and that canister serves the bytes.
    let (rt, http_gateway) = setup_http_client(pic);

    let redirect = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id,
                canister_request: Request::builder()
                    .uri(upload_path.as_str())
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    assert_eq!(
        redirect.canister_response.status(),
        307,
        "the collection must redirect the upload path to its storage canister"
    );

    let location = redirect
        .canister_response
        .headers()
        .get("location")
        .expect("redirect must carry a location")
        .to_str()
        .unwrap()
        .to_string();

    let storage_canister_id = Principal::from_str(
        location
            .split('.')
            .next()
            .unwrap()
            .replace("http://", "")
            .replace("https://", "")
            .as_str(),
    )
    .unwrap();
    assert_ne!(
        storage_canister_id, collection_canister_id,
        "the file must be stored in a storage sub-canister, not in the collection"
    );

    // File bytes are only served on the raw domain; a certified-domain request
    // redirects there rather than serving.
    let served = raw_get(&rt, &http_gateway, storage_canister_id, &location);
    assert_eq!(
        served.canister_response.status(),
        200,
        "the storage canister must serve a file uploaded without a declared hash"
    );

    rt.block_on(async {
        let body = served
            .canister_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        assert_eq!(
            body, data,
            "the bytes served back must be the bytes uploaded"
        );
    });
}
