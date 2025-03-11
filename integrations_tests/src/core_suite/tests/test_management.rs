use crate::client::core_nft::{
    init_upload,
    store_chunk,
    finalize_upload,
    cancel_upload,
    delete_file,
};
use candid::{ Nat, Principal };

use http::StatusCode;
use ic_cdk::println;
use reqwest::blocking::ClientBuilder;
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;
use storage_api_canister::finalize_upload;
use storage_api_canister::cancel_upload;
use storage_api_canister::delete_file;
use sha2::{ Sha256, Digest };

use crate::core_suite::setup::setup::TestEnv;
use crate::core_suite::setup::default_test_setup;
use std::fs::File;
use std::io::Read;
use std::net::{ IpAddr, Ipv4Addr, SocketAddr };
use std::path::Path;
use std::str::FromStr;
use ic_http_gateway::{ HttpGatewayClient, HttpGatewayRequestArgs };
use http::Request;
use ic_agent::Agent;
use bytes::Bytes;
use http_body_util::BodyExt;

#[test]
fn test_storage_simple() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

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
        })
    );

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    while offset < buffer.len() {
        let chunk = &buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())];
        let store_chunk_resp = store_chunk(
            pic,
            controller,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            })
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
        })
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

    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = pic.auto_progress();
    println!("url: {:?}", url);
    println!(
        "request : {:?}",
        Request::builder().uri(format!("/test.png").as_str()).body(Bytes::new()).unwrap()
    );

    let agent = Agent::builder().with_url(url).build().unwrap();
    rt.block_on(async {
        agent.fetch_root_key().await.unwrap();
    });
    let http_gateway = HttpGatewayClient::builder().with_agent(agent).build().unwrap();

    let response = rt.block_on(async { http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(format!("/test.png").as_str())
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send().await });

    let response_headers = response.canister_response
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
                location_str.split('.').next().unwrap().replace("https://", "").as_str()
            ).unwrap();

            let first_redirected_response = rt.block_on(async { http_gateway
                    .request(HttpGatewayRequestArgs {
                        canister_id: canister_id,
                        canister_request: Request::builder()
                            .uri(location_str)
                            .body(Bytes::new())
                            .unwrap(),
                    })
                    .send().await });

            let first_redirected_response_headers = first_redirected_response.canister_response
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str(), v.to_str().unwrap()))
                .collect::<Vec<(&str, &str)>>();

            println!("redirected_response_headers: {:?}", first_redirected_response_headers);
            println!(
                "redirected_response status: {:?}",
                first_redirected_response.canister_response.status()
            );
            if first_redirected_response.canister_response.status() == 307 {
                if let Some(location_bis) = response.canister_response.headers().get("location") {
                    let location_str = location_bis.to_str().unwrap();
                    let canister_id = Principal::from_str(
                        location_str.split('.').next().unwrap().replace("https://", "").as_str()
                    ).unwrap();

                    let second_redirected_response = rt.block_on(async { http_gateway
                            .request(HttpGatewayRequestArgs {
                                canister_id: canister_id,
                                canister_request: Request::builder()
                                    .uri(location_str)
                                    .body(Bytes::new())
                                    .unwrap(),
                            })
                            .send().await });

                    let second_redirected_response_headers =
                        second_redirected_response.canister_response
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
                        let body = second_redirected_response.canister_response
                            .into_body()
                            .collect().await
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

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    // First upload attempt
    let _ = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        })
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
            })
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
        })
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

    // Second upload attempt with the same file
    let init_upload_resp_2 = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        })
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

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

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
        })
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
            })
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
            })
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
        })
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

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

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
        })
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
            })
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
        })
    );

    match finalize_upload_resp {
        Ok(_) => {
            println!("Finalize upload should not be allowed with missing chunk");
            assert!(false);
        }
        Err(e) => {
            println!("Expected error on finalize upload with missing chunk: {:?}", e);
            assert!(true);
        }
    }
}

#[test]
fn test_cancel_upload() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

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
        })
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
        })
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
        })
    );

    match finalize_upload_resp {
        Ok(_) => {
            println!("Finalize upload should not be allowed for a canceled upload");
            assert!(false);
        }
        Err(e) => {
            println!("Expected error on finalize upload for a canceled upload: {:?}", e);
            assert!(true);
        }
    }
}

#[test]
fn test_delete_file() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

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
            file_path: "/test_delete.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        })
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
                file_path: "/test_delete.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            })
        );

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test_delete.png".to_string(),
        })
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

    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = pic.auto_progress();
    println!("url: {:?}", url);

    let agent = Agent::builder().with_url(url).build().unwrap();
    rt.block_on(async {
        agent.fetch_root_key().await.unwrap();
    });
    let http_gateway = HttpGatewayClient::builder().with_agent(agent).build().unwrap();

    // Initial request to get the file
    let response = rt.block_on(async { http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(format!("/test_delete.png").as_str())
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send().await });

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
        })
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
    let response = rt.block_on(async { http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(format!("/test_delete.png").as_str())
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send().await });

    match response.canister_response.status() {
        StatusCode::OK | StatusCode::TEMPORARY_REDIRECT => {
            panic!("File should not be found after deletion");
        }
        _ => {
            println!("File not found or server error");
        }
    }
}
