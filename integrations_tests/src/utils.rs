use crate::client::core_nft::mint;
use crate::client::storage::{finalize_upload, init_upload, store_chunk};
use crate::core_suite::setup::setup::MINUTE_IN_MS;

use bity_ic_storage_canister_api::{finalize_upload, init_upload, store_chunk};
use bity_ic_types::Cycles;
use bytes::Bytes;
use candid::{Nat, Principal};
use core_nft::types::management::mint::{Args as MintArgs, MintRequest, Response as MintResponse};
use http::Request;
use http_body_util::BodyExt;
use ic_agent::Agent;
use ic_http_gateway::{HttpGatewayClient, HttpGatewayRequestArgs};
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc1::account::Account;
use pocket_ic::PocketIc;
use rand::{rng, RngCore};
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
    owner: Account,
    controller: Principal,
    collection_canister_id: Principal,
    metadata: Vec<(String, ICRC3Value)>,
) -> MintResponse {
    let mint_args: MintArgs = MintArgs {
        mint_requests: vec![MintRequest {
            token_owner: owner,
            memo: Some(serde_bytes::ByteBuf::from("memo")),
            metadata,
        }],
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

pub fn create_default_metadata() -> Vec<(String, ICRC3Value)> {
    vec![
        ("name".to_string(), ICRC3Value::Text("test".to_string())),
        (
            "description".to_string(),
            ICRC3Value::Text("test".to_string()),
        ),
        ("test".to_string(), ICRC3Value::Text("test".to_string())),
    ]
}

pub fn create_default_icrc97_metadata(url: Url) -> Vec<(String, ICRC3Value)> {
    vec![(
        "icrc97:metadata".to_string(),
        ICRC3Value::Array(vec![ICRC3Value::Text(url.to_string())]),
    )]
}
