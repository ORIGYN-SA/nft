use crate::client::core_nft::mint;
use crate::client::storage::{finalize_upload, init_upload, store_chunk};
use crate::core_suite::setup::setup::{TestEnv, MINUTE_IN_MS};

use bity_ic_storage_canister_api::{finalize_upload, init_upload, store_chunk};
use bity_ic_types::Cycles;
use bytes::Bytes;
use candid::{CandidType, Nat, Principal};
use core_nft::types::management::mint::{Args as MintArgs, Response as MintResponse};
use http::Request;
use http_body_util::BodyExt;
use ic_agent::Agent;
use ic_http_gateway::{HttpGatewayClient, HttpGatewayRequestArgs};
use icrc_ledger_types::icrc1::account::Account;
use pocket_ic::{PocketIc, RejectResponse};
use rand::{rng, RngCore};
use serde::de::DeserializeOwned;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tempfile::NamedTempFile;
use url::Url;

pub fn random_principal() -> Principal {
    let mut bytes = [0u8; 29];
    rng().fill_bytes(&mut bytes);
    Principal::from_slice(&bytes)
}

pub fn tick_n_blocks(pic: &PocketIc, times: u32) {
    for _ in 0..times {
        pic.tick();
    }
}

pub fn mint_nft(
    pic: &mut PocketIc,
    token_name: String,
    owner: Account,
    controller: Principal,
    collection_canister_id: Principal,
) -> MintResponse {
    let metadata_json = json!({
        "description": "test",
        "name": token_name.clone(),
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
                "trait_type": "test4",
                "value": 1.4,
                "display_type": "number"
            },
            {
                "display_type": "boost_percentage",
                "trait_type": "test10",
                "value": 10
            },
            {
                "display_type": "test3",
                "trait_type": "Generation",
                "value": 2
            }
        ]
    });

    let metadata_url =
        upload_metadata(pic, controller, collection_canister_id, metadata_json).unwrap();

    let mint_args: MintArgs = MintArgs {
        token_name: token_name,
        token_metadata_url: metadata_url.to_string(),
        token_owner: owner,
        memo: Some(serde_bytes::ByteBuf::from("memo")),
    };

    let mint_call = mint(pic, controller, collection_canister_id, &mint_args);

    pic.tick();
    pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 30));

    return mint_call;
}

pub fn upload_file(
    pic: &mut PocketIc,
    controller: Principal,
    storage_canister_id: Principal,
    file_path: &str,
    upload_path: &str,
) -> Result<Vec<u8>, String> {
    let file_path = Path::new(file_path);
    let mut file = File::open(&file_path).map_err(|e| format!("Failed to open file: {:?}", e))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to read file: {:?}", e))?;

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let init_upload_resp = init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: upload_path.to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        }),
    )
    .map_err(|e| format!("init_upload error: {:?}", e))?;

    println!("init_upload_resp: {:?}", init_upload_resp);

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    while offset < buffer.len() {
        let chunk = &buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())];
        let store_chunk_resp = store_chunk(
            pic,
            controller,
            storage_canister_id,
            &(store_chunk::Args {
                file_path: upload_path.to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            }),
        )
        .map_err(|e| format!("store_chunk error: {:?}", e))?;

        println!("store_chunk_resp: {:?}", store_chunk_resp);

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: upload_path.to_string(),
        }),
    )
    .map_err(|e| format!("finalize_upload error: {:?}", e))?;

    println!("finalize_upload_resp: {:?}", finalize_upload_resp);

    Ok(buffer)
}

pub fn upload_metadata(
    pic: &mut PocketIc,
    controller: Principal,
    storage_canister_id: Principal,
    metadata: serde_json::Value,
) -> Result<Url, String> {
    println!("metadata: {:?}", metadata);
    let metadata_json_str = serde_json::to_string_pretty(&metadata).unwrap();

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", metadata_json_str).expect("Failed to write to temp file");

    println!("metadata_json_str: {}", metadata_json_str);

    let mut hasher = Sha256::new();
    hasher.update(metadata_json_str.as_bytes());
    let file_hash = hasher.finalize();
    let hash_string = format!("{:x}", file_hash);

    let upload_path = format!("{}.json", hash_string);

    upload_file(
        pic,
        controller,
        storage_canister_id,
        temp_file.path().to_str().unwrap(),
        &upload_path,
    )
    .map_err(|e| format!("upload_file error: {:?}", e))?;

    Ok(Url::parse(&format!(
        "https://{}.raw.icp0.io/{}",
        storage_canister_id, upload_path
    ))
    .unwrap())
}

pub const T: Cycles = 1_000_000_000_000;

// Helper function to setup HTTP client
pub fn setup_http_client(pic: &mut PocketIc) -> (tokio::runtime::Runtime, HttpGatewayClient) {
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

    (rt, http_gateway)
}

// Helper function to extract file path from metadata URL
pub fn extract_metadata_file_path(metadata_url: &Url) -> String {
    let metadata_file_path = metadata_url
        .to_string()
        .split("://")
        .nth(1)
        .unwrap_or(&metadata_url.to_string())
        .split('/')
        .skip(1)
        .collect::<Vec<&str>>()
        .join("/");
    format!("/{}", metadata_file_path)
}

// Helper function to fetch JSON metadata via HTTP with redirections
pub fn fetch_metadata_json(
    rt: &tokio::runtime::Runtime,
    http_gateway: &HttpGatewayClient,
    collection_canister_id: Principal,
    metadata_file_path: &str,
) -> serde_json::Value {
    println!("metadata_file_path : {}", metadata_file_path);

    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(metadata_file_path)
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    assert_eq!(
        response.canister_response.status(),
        307,
        "should return a redirection"
    );

    if let Some(location) = response.canister_response.headers().get("location") {
        let location_str = location.to_str().unwrap();
        println!("Redirection to: {}", location_str);

        let canister_id = Principal::from_str(
            location_str
                .split('.')
                .next()
                .unwrap()
                .replace("https://", "")
                .as_str(),
        )
        .unwrap();

        let redirected_response = rt.block_on(async {
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

        println!(
            "Status of the first redirection: {}",
            redirected_response.canister_response.status()
        );

        if redirected_response.canister_response.status() == 307 {
            if let Some(location_bis) = redirected_response
                .canister_response
                .headers()
                .get("location")
            {
                let location_str = location_bis.to_str().unwrap();
                println!("Second redirection to: {}", location_str);

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

                assert_eq!(
                    second_redirected_response.canister_response.status(),
                    200,
                    "should retrieve the file with success"
                );

                return rt.block_on(async {
                    let body = second_redirected_response
                        .canister_response
                        .into_body()
                        .collect()
                        .await
                        .unwrap()
                        .to_bytes()
                        .to_vec();

                    let json_content =
                        String::from_utf8(body).expect("The content should be valid JSON");
                    println!("Retrieved JSON content: {}", json_content);

                    serde_json::from_str(&json_content).expect("The JSON should be parsable")
                });
            }
        } else if redirected_response.canister_response.status() == 200 {
            return rt.block_on(async {
                let body = redirected_response
                    .canister_response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .to_vec();

                let json_content =
                    String::from_utf8(body).expect("The content should be valid JSON");
                println!("Retrieved JSON content: {}", json_content);

                serde_json::from_str(&json_content).expect("The JSON should be parsable")
            });
        } else {
            panic!(
                "Unexpected status: {}",
                redirected_response.canister_response.status()
            );
        }
    }

    panic!("No location header found in redirection response");
}

/// Helper function to test sliding window rate limiting
/// This function attempts to make multiple calls to a protected endpoint
/// and verifies that the rate limiting works correctly
/// Note: When rate limited, the canister will trap, so we need to handle that
pub fn test_sliding_window_rate_limit<T, E>(
    pic: &mut PocketIc,
    caller: Principal,
    canister_id: Principal,
    call_function: impl Fn(&mut PocketIc, Principal, Principal) -> Result<Vec<u8>, RejectResponse>,
    max_calls: usize,
    window_duration_ms: u64,
) -> Result<(), String>
where
    Result<T, E>: CandidType + DeserializeOwned + std::fmt::Debug,
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    // Make calls up to the limit
    for i in 0..max_calls {
        let result = call_function(pic, caller, canister_id);
        match result {
            Ok(result) => {
                let decoded_res: Result<T, E> = candid::decode_one(&result).unwrap();
                match decoded_res {
                    Ok(result) => {}
                    Err(error) => {
                        return Err(format!("Should have trapped: {:?}", error));
                    }
                }
            }
            Err(error) => {
                return Err(error.to_string());
            }
        }

        // Advance time slightly between calls to ensure they're recorded
        pic.advance_time(Duration::from_nanos(1_000));
        tick_n_blocks(pic, 5);
    }

    // Try one more call - this should trap due to rate limiting
    let result = call_function(pic, caller, canister_id);

    match result {
        Ok(result) => {
            let decoded_res: Result<T, E> = candid::decode_one(&result).unwrap();
            match decoded_res {
                Ok(result) => {
                    // should have trapped
                    return Err(format!("Should have trapped: {:?}", result));
                }
                Err(error) => {
                    // should have trapped
                    return Err(format!("Should have trapped: {:?}", error));
                }
            }
        }
        Err(error) => {
            // ok, expected to trap
        }
    }

    // Advance time beyond the window duration
    pic.advance_time(Duration::from_millis(window_duration_ms + 1000));
    tick_n_blocks(pic, 10);

    // Try another call - this should succeed again
    let result = call_function(pic, caller, canister_id);

    match result {
        Ok(result) => {
            let decoded_res: Result<T, E> = candid::decode_one(&result).unwrap();
            match decoded_res {
                Ok(result) => {}
                Err(error) => {
                    return Err(format!("Should have trapped: {:?}", error));
                }
            }
        }
        Err(error) => {
            return Err(error.to_string());
        }
    }

    Ok(())
}

/// Helper function to test that different users have independent rate limits
pub fn test_sliding_window_multiple_users<T, E>(
    pic: &mut PocketIc,
    user1: Principal,
    user2: Principal,
    canister_id: Principal,
    call_function: impl Fn(&mut PocketIc, Principal, Principal) -> Result<Vec<u8>, RejectResponse>,
    max_calls: usize,
) -> Result<(), String>
where
    Result<T, E>: CandidType + DeserializeOwned + std::fmt::Debug,
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    // User 1 makes calls up to the limit
    for _ in 0..max_calls {
        let result = call_function(pic, user1, canister_id);
        match result {
            Ok(result) => {
                let decoded_res: Result<T, E> = candid::decode_one(&result).unwrap();
                match decoded_res {
                    Ok(result) => {}
                    Err(error) => {
                        return Err(format!("Should have trapped: {:?}", error));
                    }
                }
            }
            Err(error) => {
                return Err(error.to_string());
            }
        }

        pic.advance_time(Duration::from_millis(100));
        tick_n_blocks(pic, 1);
    }

    // User 2 should still be able to make calls (independent limits)
    for _ in 0..max_calls {
        let result = call_function(pic, user2, canister_id);
        match result {
            Ok(result) => {
                let decoded_res: Result<T, E> = candid::decode_one(&result).unwrap();
                match decoded_res {
                    Ok(result) => {}
                    Err(error) => {
                        return Err(format!("Should have trapped: {:?}", error));
                    }
                }
            }
            Err(error) => {
                return Err(error.to_string());
            }
        }

        pic.advance_time(Duration::from_millis(100));
        tick_n_blocks(pic, 1);
    }

    // User 1's next call should fail
    let result = call_function(pic, user1, canister_id);
    match result {
        Ok(result) => {
            let decoded_res: Result<T, E> = candid::decode_one(&result).unwrap();
            match decoded_res {
                Ok(result) => {
                    return Err(format!("Should have trapped: {:?}", result));
                }
                Err(error) => {
                    return Err(format!("Should have trapped: {:?}", error));
                }
            }
        }
        Err(error) => {
            return Err(error.to_string());
        }
    }
    // User 2's next call should also fail
    let result = call_function(pic, user2, canister_id);
    match result {
        Ok(result) => {
            let decoded_res: Result<T, E> = candid::decode_one(&result).unwrap();
            match decoded_res {
                Ok(result) => {
                    return Err(format!("Should have trapped: {:?}", result));
                }
                Err(error) => {
                    return Err(format!("Should have trapped: {:?}", error));
                }
            }
        }
        Err(error) => {
            return Err(error.to_string());
        }
    }

    Ok(())
}
