use crate::client::core_nft::{
    cancel_upload, finalize_upload, get_upload_status, get_user_permissions, grant_permission,
    icrc7_token_metadata, init_upload, mint, revoke_permission, store_chunk,
    update_collection_metadata, update_nft_metadata,
};
use crate::utils::{create_default_icrc97_metadata, create_default_metadata};

use candid::{Encode, Nat, Principal};
use core_nft::types::permissions::Permission;
use icrc_ledger_types::icrc1::account::Account;

use bity_ic_storage_canister_api::types::storage::UploadState;
use core_nft::types::management::{
    cancel_upload, finalize_upload, get_upload_status, get_user_permissions, grant_permission,
    init_upload, mint, revoke_permission, store_chunk, update_collection_metadata,
    update_nft_metadata,
};
use ic_cdk::println;
use sha2::{Digest, Sha256};

use crate::core_suite::setup::default_test_setup;
use crate::core_suite::setup::setup::TestEnv;
use crate::utils::{
    extract_metadata_file_path, fetch_metadata_json, setup_http_client, upload_file,
    upload_metadata,
};
use bytes::Bytes;
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

    if response.canister_response.status() == 307 {
        if let Some(location) = response.canister_response.headers().get("location") {
            let location_str = location.to_str().unwrap();
            let canister_id = Principal::from_str(
                location_str
                    .split('.')
                    .next()
                    .unwrap()
                    .replace("https://", "")
                    .as_str(),
            )
            .unwrap();

            let first_redirected_response = rt.block_on(async {
                http_gateway
                    .request(HttpGatewayRequestArgs {
                        canister_id: canister_id,
                        canister_request: Request::builder()
                            .uri(location_str)
                            .body(Bytes::new())
                            .unwrap(),
                    })
                    .send()
                    .await
            });

            let first_redirected_response_headers = first_redirected_response
                .canister_response
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str(), v.to_str().unwrap()))
                .collect::<Vec<(&str, &str)>>();

            println!(
                "redirected_response_headers: {:?}",
                first_redirected_response_headers
            );
            println!(
                "redirected_response status: {:?}",
                first_redirected_response.canister_response.status()
            );
            if first_redirected_response.canister_response.status() == 307 {
                if let Some(location_bis) = first_redirected_response
                    .canister_response
                    .headers()
                    .get("location")
                {
                    let location_str = location_bis.to_str().unwrap();
                    let canister_id = Principal::from_str(
                        location_str
                            .split('.')
                            .next()
                            .unwrap()
                            .replace("https://", "")
                            .as_str(),
                    )
                    .unwrap();

                    let second_redirected_response = rt.block_on(async {
                        http_gateway
                            .request(HttpGatewayRequestArgs {
                                canister_id: canister_id,
                                canister_request: Request::builder()
                                    .uri(location_str)
                                    .body(Bytes::new())
                                    .unwrap(),
                            })
                            .send()
                            .await
                    });

                    let second_redirected_response_headers = second_redirected_response
                        .canister_response
                        .headers()
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.to_str().unwrap()))
                        .collect::<Vec<(&str, &str)>>();

                    println!(
                        "redirected_response_headers: {:?}",
                        second_redirected_response_headers
                    );
                    println!(
                        "redirected_response status: {:?}",
                        second_redirected_response.canister_response.status()
                    );

                    rt.block_on(async {
                        let body = second_redirected_response
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
            }
        }
    } else {
        panic!("Expected 307 status code");
    }
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
            file_hash: "dummy_hash".to_string(),
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
            file_hash: format!("{:x}", file_hash),
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
            file_hash: format!("{:x}", file_hash),
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
    for (upload_path, original_buffer) in uploaded_files {
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
            let canister_id = Principal::from_str(
                location_str
                    .split('.')
                    .next()
                    .unwrap()
                    .replace("https://", "")
                    .as_str(),
            )
            .unwrap();

            canister_distribution
                .entry(canister_id.to_string())
                .or_insert_with(Vec::new)
                .push(upload_path.clone());
        }
    }

    // Verify that files are distributed evenly (2 files per canister)
    for (canister_id, files) in &canister_distribution {
        assert_eq!(
            files.len(),
            7,
            "Canister {} should contain exactly 2 files, but has {}",
            canister_id,
            files.len()
        );
    }

    // Verify we have exactly 2 canisters
    assert_eq!(
        canister_distribution.len(),
        2,
        "Should have exactly 2 canisters, but found {}",
        canister_distribution.len()
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
                .replace("https://", "")
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
                .replace("https://", "")
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
        let first_storage_canister = Principal::from_str(
            location_str
                .split('.')
                .next()
                .unwrap()
                .replace("https://", "")
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
                    .replace("https://", "")
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

    // Verify each storage canister has sufficient cycles
    for (canister_id, cycles) in &canister_cycles {
        assert!(
            *cycles >= 1_000_000_000_000, // 1T cycles minimum threshold
            "Storage canister {} has insufficient cycles: {}",
            canister_id,
            cycles
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
            file_hash: "dummy_hash".to_string(),
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
            metadata: create_default_icrc97_metadata(metadata_url),
            token_owner: Account {
                owner: nft_owner1,
                subaccount: None,
            },
            memo: None,
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
            metadata: create_default_icrc97_metadata(metadata_url.clone()),
            token_owner: Account {
                owner: nft_owner1,
                subaccount: None,
            },
            memo: None,
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
            metadata: create_default_icrc97_metadata(metadata_url),
            token_owner: Account {
                owner: nft_owner1,
                subaccount: None,
            },
            memo: None,
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
            metadata: create_default_icrc97_metadata(metadata_url.clone()),
            token_owner: Account {
                owner: nft_owner1,
                subaccount: None,
            },
            memo: None,
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
            Err(core_nft::types::management::get_upload_status::GetUploadStatusError::UploadNotFound)
        ),
        "Should return error for non-existent upload"
    );

    let init_upload_resp = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: upload_path2.to_string(),
            file_hash: "dummy_hash".to_string(),
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

    let all_uploads: core_nft::types::management::get_all_uploads::Response =
        crate::client::pocket::unwrap_response(pic.query_call(
            collection_canister_id,
            controller,
            "get_all_uploads",
            Encode!(&(), &()).unwrap(),
        ));

    assert_eq!(all_uploads.unwrap().len(), 3, "Should return all 3 uploads");

    // Test pagination
    let first_page: core_nft::types::management::get_all_uploads::Response =
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

    let second_page: core_nft::types::management::get_all_uploads::Response =
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
            max_canister_storage_threshold: Some(Nat::from(1000000u64)),
            collection_metadata: Some(HashMap::new()),
        }),
    );
    assert!(
        result.is_ok(),
        "Should update collection metadata successfully"
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
            Err(core_nft::types::management::update_collection_metadata::UpdateCollectionMetadataError::ConcurrentManagementCall)
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
            metadata: create_default_icrc97_metadata(metadata_url),
            token_owner: Account {
                owner: test_principal,
                subaccount: None,
            },
            memo: None,
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
            file_hash: "dummy_hash".to_string(),
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
