use anyhow::{anyhow, Result};
use candid::{Encode, Principal};
use clap::{arg, command, Command};
use core_nft::updates::management::{finalize_upload, init_upload, store_chunk};
use ic_agent::{identity::Secp256k1Identity, Agent};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

async fn initialize_agent(identity_pem_path: &str) -> Result<Agent> {
    let identity = Secp256k1Identity::from_pem_file(identity_pem_path)?;

    let agent = Agent::builder()
        .with_identity(identity)
        .with_url("https://ic0.app")
        .build()
        .expect("Failed to create Internet Computer agent. This should not happen.");

    agent.fetch_root_key().await?;

    Ok(agent)
}

async fn nft_init_upload(
    agent: &Agent,
    canister_id: &Principal,
    args: init_upload::Args,
) -> Result<()> {
    println!("Initializing upload...");
    println!("File: {:?}", args.file_path);
    println!("Size: {:?} bytes", args.file_size);
    println!("SHA-256 hash: {:?}", args.file_hash);
    println!("Chunk size: {:?} bytes", args.chunk_size);

    let bytes = Encode!(&args)?;
    let _ = agent
        .update(canister_id, "init_upload")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    println!("Upload initialized successfully");
    Ok(())
}

async fn nft_store_chunk(
    agent: &Agent,
    canister_id: &Principal,
    args: store_chunk::Args,
) -> Result<()> {
    println!("Uploading {} chunks...", args.chunk_id);

    let bytes = Encode!(&args)?;
    let _ = agent
        .update(canister_id, "store_chunk")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    println!("Chunk {} uploaded successfully", args.chunk_id);

    Ok(())
}

async fn nft_finalize_upload(
    agent: &Agent,
    canister_id: &Principal,
    args: finalize_upload::Args,
) -> Result<()> {
    println!("Finalizing upload...");

    let bytes = Encode!(&args)?;
    let _ = agent
        .update(canister_id, "finalize_upload")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    println!("Upload finalized successfully");
    Ok(())
}

async fn upload_file(
    agent: &Agent,
    canister_id: &Principal,
    file_path: &str,
    destination_path: &str,
    chunk_size: u64,
) -> Result<()> {
    let mut file = File::open(file_path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let hash = Sha256::digest(&buffer);
    let file_hash = format!("{:x}", hash);

    let mut file = File::open(file_path)?;

    nft_init_upload(
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

        nft_store_chunk(
            &agent,
            &canister_id,
            store_chunk::Args {
                chunk_id: candid::Nat::from(i),
                chunk_data,
                file_path: destination_path.to_string(),
            },
        )
        .await?;
    }

    nft_finalize_upload(
        &agent,
        &canister_id,
        finalize_upload::Args {
            file_path: destination_path.to_string(),
        },
    )
    .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("ICRC7 File Upload Tool")
        .version("1.0")
        .author("Gautier Wojda <gautier.wojda@bity.com>")
        .about("Upload files to ICRC7 canister")
        .subcommand(
            Command::new("upload-file")
                .about("Upload a file to the canister")
                .arg(arg!(<canister_id> "Target canister ID"))
                .arg(arg!(<file_path> "Path to the file to upload"))
                .arg(arg!(<destination_path> "Destination path in the canister (e.g. /images/photo.jpg)"))
                .arg(arg!(<identity> "Path to the identity PEM file")),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("upload-file", sub_matches)) => {
            let canister_id_str = sub_matches.get_one::<String>("canister_id").unwrap();
            let file_path = sub_matches.get_one::<String>("file_path").unwrap();
            let destination_path = sub_matches.get_one::<String>("destination_path").unwrap();
            let identity_path = sub_matches.get_one::<String>("identity").unwrap();

            // Check if file exists
            if !Path::new(file_path).exists() {
                return Err(anyhow!("File '{}' does not exist", file_path));
            }

            let canister_id = Principal::from_text(canister_id_str)?;
            let chunk_size: u64 = 1 * 1024 * 1024; // 1MB chunks
            let agent = initialize_agent(identity_path).await?;

            upload_file(
                &agent,
                &canister_id,
                file_path,
                destination_path,
                chunk_size,
            )
            .await?;

            println!("File upload completed successfully!");
        }
        _ => {
            println!("Please specify a valid subcommand. Use --help for more information.");
            return Ok(());
        }
    }

    Ok(())
}
