use crate::client::core_nft::mint;
use crate::client::storage::{finalize_upload, init_upload, store_chunk};
use crate::core_suite::setup::setup::{TestEnv, MINUTE_IN_MS};
use bity_ic_types::Cycles;
use candid::{Nat, Principal};
use core_nft::types::management::mint::{Args as MintArgs, Response as MintResponse};
use icrc_ledger_types::icrc1::account::Account;
use pocket_ic::PocketIc;
use rand::{rng, RngCore};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;
use storage_api_canister::finalize_upload;
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;

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
    let mint_args: MintArgs = MintArgs {
        token_name: token_name,
        token_description: Some("description".to_string()),
        token_logo: Some("logo".to_string()),
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

pub const T: Cycles = 1_000_000_000_000;
