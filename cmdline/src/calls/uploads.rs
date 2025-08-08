use anyhow::Result;
use candid::{Encode, Nat, Principal};
use core_nft::updates::management::{finalize_upload, init_upload, store_chunk};
use ic_agent::Agent;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use url::Url;

pub async fn init(agent: &Agent, canister_id: &Principal, args: init_upload::Args) -> Result<()> {
    let bytes = Encode!(&args)?;
    agent
        .update(canister_id, "init_upload")
        .with_arg(bytes)
        .call_and_wait()
        .await?;
    Ok(())
}

pub async fn store(
    agent: &Agent,
    canister_id: &Principal,
    args: store_chunk::Args,
) -> Result<()> {
    let bytes = Encode!(&args)?;
    agent
        .update(canister_id, "store_chunk")
        .with_arg(bytes)
        .call_and_wait()
        .await?;
    Ok(())
}

pub async fn finalize(
    agent: &Agent,
    canister_id: &Principal,
    args: finalize_upload::Args,
) -> Result<Url> {
    let bytes = Encode!(&args)?;
    let response = agent
        .update(canister_id, "finalize_upload")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    let url = candid::decode_one::<finalize_upload::Response>(&response)?
        .map_err(|e| anyhow::anyhow!("Finalize upload failed: {:?}", e))?;

    Ok(Url::parse(&url.url)?)
}

pub async fn upload_file(
    agent: &Agent,
    canister_id: &Principal,
    file_path: &str,
    destination_path: &str,
    chunk_size: u64,
) -> Result<Url> {
    let mut file = File::open(file_path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let hash = Sha256::digest(&buffer);
    let file_hash = format!("{:x}", hash);

    let mut file = File::open(file_path)?;

    init(
        &agent,
        &canister_id,
        init_upload::Args {
            file_path: destination_path.to_string(),
            file_size,
            chunk_size: Some(chunk_size),
            file_hash,
        },
    )
    .await?;

    let total_chunks = (file_size + chunk_size - 1) / chunk_size;

    for i in 0..total_chunks {
        let mut chunk_data = vec![0; chunk_size as usize];
        let bytes_read = file.read(&mut chunk_data)?;
        if bytes_read == 0 {
            break;
        }

        chunk_data.truncate(bytes_read);

        store(
            &agent,
            &canister_id,
            store_chunk::Args {
                chunk_id: Nat::from(i),
                chunk_data,
                file_path: destination_path.to_string(),
            },
        )
        .await?;
    }

    let url = finalize(
        &agent,
        &canister_id,
        finalize_upload::Args {
            file_path: destination_path.to_string(),
        },
    )
    .await?;

    Ok(url)
}


