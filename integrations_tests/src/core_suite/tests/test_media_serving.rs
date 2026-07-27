use crate::client::core_nft::{
    finalize_private_content_upload, finalize_upload, init_private_content_upload, init_upload,
    store_chunk, store_private_content_chunk,
};
use crate::core_suite::setup::setup::TestEnv;
use crate::core_suite::setup::setup_core::upgrade_core_canister;
use crate::core_suite::setup::{default_test_setup, old_test_setup};
use crate::utils::{setup_http_client, tick_n_blocks};
use bity_ic_types::BuildVersion;
use bytes::Bytes;
use candid::{Nat, Principal};
use core_nft_api::lifecycle::Args;
use core_nft_api::post_upgrade::UpgradeArgs;
use core_nft_common::types::management::{finalize_upload, init_upload, store_chunk};
use core_nft_common::EncryptionMode;
use serde_bytes::ByteBuf;
use std::collections::HashMap;
use http::Request;
use ic_http_gateway::HttpGatewayRequestArgs;
use pocket_ic::PocketIc;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

fn upload_file_as(
    pic: &mut PocketIc,
    sender: Principal,
    collection_canister_id: Principal,
    file_path: &str,
    buffer: &[u8],
) {
    let mut hasher = Sha256::new();
    hasher.update(buffer);
    let file_hash = hasher.finalize();

    init_upload(
        pic,
        sender,
        collection_canister_id,
        &(init_upload::Args {
            file_path: file_path.to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size: buffer.len() as u64,
            chunk_size: None,
        }),
    )
    .expect("init_upload failed");

    for (chunk_index, chunk) in buffer.chunks(1024 * 1024).enumerate() {
        store_chunk(
            pic,
            sender,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: file_path.to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            }),
        )
        .expect("store_chunk failed");
    }

    finalize_upload(
        pic,
        sender,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: file_path.to_string(),
        }),
    )
    .expect("finalize_upload failed");
}

fn test_asset() -> Vec<u8> {
    let mut file =
        File::open(Path::new("./src/core_suite/assets/test.png")).expect("Failed to open asset");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read asset");
    buffer
}

/// A path this collection has never heard of must answer 404.
///
/// It used to `trap("Failed to serve asset")`, which the boundary node
/// surfaces as a 503. That is indistinguishable from the canister being down,
/// so a typo in a metadata URL looked like an outage.
#[test]
fn test_unknown_path_is_not_found_rather_than_a_trap() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        ..
    } = test_env;

    let (rt, http_gateway) = setup_http_client(pic);

    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: collection_canister_id,
                canister_request: Request::builder()
                    .uri("/there-is-no-such-file.png")
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    assert_eq!(
        response.canister_response.status(),
        404,
        "an unknown path must 404, not trap"
    );
}

/// A file uploaded by a previous-generation wasm must still be served after the
/// collection is upgraded.
///
/// A guard on the existing redirect replay, not on the new fallback: this
/// passes on the pre-fix build too. It is here because the upgrade path rebuilds
/// the asset router from scratch every time, and a regression there would take
/// every legacy file offline at once.
#[test]
fn test_legacy_upload_is_still_served_after_upgrade() {
    let mut test_env: TestEnv = old_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    let buffer = test_asset();
    upload_file_as(pic, controller, collection_canister_id, "/test.png", &buffer);

    upgrade_core_canister(
        pic,
        collection_canister_id,
        Args::Upgrade(UpgradeArgs {
            version: BuildVersion::min(),
            commit_hash: "media serving test".to_string(),
            vetkd_key_name: None,
            vetkd_context: None,
            base_url: Some("https://{canister_id}.raw.icp0.io".to_string()),
        }),
        controller,
    );

    pic.advance_time(Duration::from_secs(1));
    tick_n_blocks(pic, 30);

    let (rt, http_gateway) = setup_http_client(pic);

    let get = |path: &str| {
        let uri = path.to_string();
        rt.block_on(async {
            http_gateway
                .request(HttpGatewayRequestArgs {
                    canister_id: collection_canister_id,
                    canister_request: Request::builder()
                        .uri(uri.as_str())
                        .body(Bytes::new())
                        .unwrap(),
                })
                .send()
                .await
        })
    };

    let response = get("/test.png");
    assert_eq!(
        response.canister_response.status(),
        307,
        "a legacy upload must resolve to its storage canister after an upgrade"
    );

    let location = response
        .canister_response
        .headers()
        .get("location")
        .expect("redirect must carry a location")
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        location.ends_with("/test.png") && location.contains(".raw.icp0.io"),
        "unexpected redirect target: {location}"
    );
    assert!(
        !location.contains(&collection_canister_id.to_text()),
        "must redirect to the storage canister, not back to itself: {location}"
    );

    // Resolving the miss registers the redirect, so the second request is
    // answered by the router and the fix survives the next upgrade.
    let again = get("/test.png");
    assert_eq!(again.canister_response.status(), 307);
    assert_eq!(
        again
            .canister_response
            .headers()
            .get("location")
            .expect("redirect must carry a location")
            .to_str()
            .unwrap(),
        location,
        "the self-healed redirect must point at the same target"
    );
}

/// Private content uploaded without a leading slash must still be served, and
/// must survive an upgrade.
///
/// Two separate defects, both on this path. `finalize_private_content_upload`
/// registered the redirect under the raw `storage_path`, but the asset router
/// keys on the exact string and `HttpRequest::get_path` always leads with a
/// slash, so `private.bin` could never match a request for `/private.bin`. And
/// it never wrote `media_redirections`, the only thing `post_upgrade` rebuilds
/// the router from, so even a correctly keyed redirect died at the next upgrade.
#[test]
fn test_private_content_is_served_and_survives_upgrade() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    let content = b"0123456789abcdef".to_vec();
    let hash_bytes = Sha256::digest(&content);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);

    // Deliberately no leading slash: this is what the router used to be keyed by.
    let storage_path = "private_no_leading_slash.bin".to_string();

    init_private_content_upload(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::init_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            plaintext_hash: hash,
            file_hash: hash,
            salt: vec![],
            readers: HashMap::new(),
            default_readers: HashMap::new(),
            storage_canister_id: collection_canister_id,
            storage_path: storage_path.clone(),
            plaintext_size: content.len() as u64,
            expected_chunks: 1,
            chunk_size: Some(content.len() as u64),
            file_size: content.len() as u64,
            encryption_mode: EncryptionMode::AES256,
        },
    )
    .expect("init_private_content_upload failed");

    store_private_content_chunk(
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
    )
    .expect("store_private_content_chunk failed");

    finalize_private_content_upload(
        pic,
        controller,
        collection_canister_id,
        &core_nft_api::finalize_private_content_upload::Args {
            token_id_opt: None,
            entry_name: None,
            hash,
            storage_path: storage_path.clone(),
        },
    )
    .expect("finalize_private_content_upload failed");

    let (rt, http_gateway) = setup_http_client(pic);
    let get = |path: &str| {
        let uri = path.to_string();
        rt.block_on(async {
            http_gateway
                .request(HttpGatewayRequestArgs {
                    canister_id: collection_canister_id,
                    canister_request: Request::builder()
                        .uri(uri.as_str())
                        .body(Bytes::new())
                        .unwrap(),
                })
                .send()
                .await
        })
    };

    let response = get("/private_no_leading_slash.bin");
    assert_eq!(
        response.canister_response.status(),
        307,
        "a private upload keyed without a leading slash must still be reachable"
    );

    upgrade_core_canister(
        pic,
        collection_canister_id,
        Args::Upgrade(UpgradeArgs {
            version: BuildVersion::min(),
            commit_hash: "private content survives".to_string(),
            vetkd_key_name: None,
            vetkd_context: None,
            base_url: None,
        }),
        controller,
    );
    pic.advance_time(Duration::from_secs(1));
    tick_n_blocks(pic, 30);

    assert_eq!(
        get("/private_no_leading_slash.bin")
            .canister_response
            .status(),
        307,
        "the private redirect must survive an upgrade"
    );
}
