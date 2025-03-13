use crate::client::core_nft::{
    cancel_upload, delete_file, finalize_upload, init_upload, store_chunk,
};
use candid::{Nat, Principal};

use http::StatusCode;
use ic_cdk::println;
use sha2::{Digest, Sha256};
use storage_api_canister::cancel_upload;
use storage_api_canister::delete_file;
use storage_api_canister::finalize_upload;
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;

use crate::core_suite::setup::default_test_setup;
use crate::core_suite::setup::setup::TestEnv;
use crate::utils::upload_file;
use bytes::Bytes;
use http::Request;
use http_body_util::BodyExt;
use ic_agent::Agent;
use ic_http_gateway::{HttpGatewayClient, HttpGatewayRequestArgs};
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

    let file_path = "./src/storage_suite/assets/test.png";
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

    let file_path = "./src/storage_suite/assets/test.png";
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

    let file_path = Path::new("./src/storage_suite/assets/test.png");
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

    let file_path = Path::new("./src/storage_suite/assets/test.png");
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

    let file_path = Path::new("./src/storage_suite/assets/test.png");
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
fn test_delete_file() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/storage_suite/assets/test.png";
    let upload_path = "/test_delete.png";

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

    let agent = Agent::builder().with_url(url).build().unwrap();
    rt.block_on(async {
        agent.fetch_root_key().await.unwrap();
    });
    let http_gateway = HttpGatewayClient::builder()
        .with_agent(agent)
        .build()
        .unwrap();

    // Initial request to get the file
    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(format!("/test_delete.png").as_str())
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    match response.canister_response.status() {
        StatusCode::OK | StatusCode::TEMPORARY_REDIRECT => {
            println!("File is accessible");
        }
        _ => {
            panic!("File should be accessible");
        }
    }

    let delete_file_resp = delete_file(
        pic,
        controller,
        collection_canister_id,
        &(delete_file::Args {
            file_path: "/test_delete.png".to_string(),
        }),
    );

    match delete_file_resp {
        Ok(resp) => {
            println!("delete_file_resp: {:?}", resp);
        }
        Err(e) => {
            println!("delete_file_resp error: {:?}", e);
            assert!(false);
        }
    }

    // Attempt to get the deleted file
    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(format!("/test_delete.png").as_str())
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    match response.canister_response.status() {
        StatusCode::OK | StatusCode::TEMPORARY_REDIRECT => {
            panic!("File should not be found after deletion");
        }
        _ => {
            println!("File not found or server error");
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

    let file_path = "./src/storage_suite/assets/test.png";
    let mut uploaded_files = Vec::new();
    let mut canister_distribution = std::collections::HashMap::new();

    // Upload 4 files
    for i in 0..4 {
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
            2,
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

    let file_path = "./src/storage_suite/assets/test.png";
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

    let file_path = "./src/storage_suite/assets/test.png";
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
