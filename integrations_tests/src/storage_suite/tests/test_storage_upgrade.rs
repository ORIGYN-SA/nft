use crate::client::storage::{
    cancel_upload, delete_file, finalize_upload, get_data, get_storage_size, http_request,
    init_upload, insert_data, remove_data, store_chunk, update_data,
};
use crate::storage_suite::setup::setup_storage::upgrade_storage_canister;
use bity_ic_types::BuildVersion;
use candid::Nat;
use storage_api_canister::lifecycle::Args;
use storage_api_canister::post_upgrade::UpgradeArgs;

use http::StatusCode;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use sha2::{Digest, Sha256};
use storage_api_canister::cancel_upload;
use storage_api_canister::delete_file;
use storage_api_canister::finalize_upload;
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;
use storage_api_canister::updates::insert_data;
use storage_api_canister::updates::remove_data;
use storage_api_canister::updates::update_data;
use storage_api_canister::value_custom::CustomValue;

use crate::storage_suite::setup::setup::TestEnv;
use crate::{storage_suite::setup::default_test_setup, utils::tick_n_blocks};
use bytes::Bytes;
use http::Request;
use http_body_util::BodyExt;
use ic_agent::Agent;
use ic_http_gateway::{HttpGatewayClient, HttpGatewayRequestArgs, HttpGatewayResponseMetadata};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[test]
fn test_storage_after_update_simple() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
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
        storage_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
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
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            }),
        );

        match store_chunk_resp {
            Ok(resp) => {
                println!("store_chunk_resp: {:?}", resp);
            }
            Err(e) => {
                println!("store_chunk_resp error: {:?}", e);
            }
        }

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        storage_canister_id,
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
        }
    }

    let storage_upgrade_args = Args::Upgrade(UpgradeArgs {
        version: BuildVersion::min(),
        commit_hash: "commit_hash 2".to_string(),
    });

    upgrade_storage_canister(pic, storage_canister_id, storage_upgrade_args, controller);

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
                canister_id: storage_canister_id.clone(),
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
    let expected_headers = vec![(
        "location",
        "https://uqqxf-5h777-77774-qaaaa-cai.raw.icp0.io/test.png",
    )];

    for (key, value) in expected_headers {
        assert!(response_headers.contains(&(key, value)));
    }

    if response.canister_response.status() == 307 {
        if let Some(location) = response.canister_response.headers().get("location") {
            let location_str = location.to_str().unwrap();

            let redirected_response = rt.block_on(async {
                http_gateway
                    .request(HttpGatewayRequestArgs {
                        canister_id: storage_canister_id.clone(),
                        canister_request: Request::builder()
                            .uri(location_str)
                            .body(Bytes::new())
                            .unwrap(),
                    })
                    .send()
                    .await
            });

            let redirected_response_headers = redirected_response
                .canister_response
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str(), v.to_str().unwrap()))
                .collect::<Vec<(&str, &str)>>();

            assert!(redirected_response.canister_response.status() == 200);
            assert_eq!(redirected_response.canister_response.status(), 200);

            let expected_headers = vec![
                ("strict-transport-security", "max-age=31536000; includeSubDomains"),
                ("x-frame-options", "DENY"),
                ("x-content-type-options", "nosniff"),
                (
                    "content-security-policy",
                    "default-src 'self'; img-src 'self' data:; form-action 'self'; object-src 'none'; frame-ancestors 'none'; upgrade-insecure-requests; block-all-mixed-content",
                ),
                ("referrer-policy", "no-referrer"),
                (
                    "permissions-policy",
                    "accelerometer=(),ambient-light-sensor=(),autoplay=(),battery=(),camera=(),display-capture=(),document-domain=(),encrypted-media=(),fullscreen=(),gamepad=(),geolocation=(),gyroscope=(),layout-animations=(self),legacy-image-formats=(self),magnetometer=(),microphone=(),midi=(),oversized-images=(self),payment=(),picture-in-picture=(),publickey-credentials-get=(),speaker-selection=(),sync-xhr=(self),unoptimized-images=(self),unsized-media=(self),usb=(),screen-wake-lock=(),web-share=(),xr-spatial-tracking=()",
                ),
                ("cross-origin-embedder-policy", "require-corp"),
                ("cross-origin-opener-policy", "same-origin"),
                ("cache-control", "public, max-age=31536000, immutable"),
                ("content-type", "image/png"),
                ("content-length", "6205837")
            ];

            println!(
                "redirected_response_headers: {:?}",
                redirected_response_headers
            );
            for (key, value) in expected_headers {
                println!("key: {}, value: {}", key, value);
                assert!(redirected_response_headers.contains(&(key, value)));
            }

            rt.block_on(async {
                let body = redirected_response
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
    } else {
        panic!("Expected 307 status code");
    }
}
