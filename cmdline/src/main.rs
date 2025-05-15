use anyhow::{anyhow, Result};
use candid::{Decode, Encode, Nat, Principal};
use clap::{arg, command, Arg, Command};
use core_nft::types::management;
use core_nft::updates::management::{finalize_upload, init_upload, store_chunk};
use ic_agent::{identity::Secp256k1Identity, Agent};
use icrc_ledger_types::icrc1::account::Account;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;

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

async fn nft_mint(
    agent: &Agent,
    canister_id: &Principal,
    token_name: String,
    token_description: Option<String>,
    token_logo: Option<String>,
    owner_principal: Principal,
) -> Result<()> {
    println!("Minting new NFT...");
    println!("Token name: {}", token_name);
    if let Some(desc) = &token_description {
        println!("Description: {}", desc);
    }
    if let Some(logo) = &token_logo {
        println!("Logo: {}", logo);
    }
    println!("Owner: {}", owner_principal);

    let args = management::mint::Args {
        token_name,
        token_description,
        token_logo,
        token_owner: Account {
            owner: owner_principal,
            subaccount: None,
        },
        memo: None,
    };

    let bytes = Encode!(&args)?;
    let response = agent
        .update(canister_id, "mint")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    let token_id = Decode!(&response, Nat)?;
    println!("NFT minted successfully with token ID: {}", token_id);

    Ok(())
}

async fn nft_burn(agent: &Agent, canister_id: &Principal, token_id: Nat) -> Result<()> {
    println!("Burning NFT with token ID: {}...", token_id);

    let bytes = Encode!(&token_id)?;
    let _ = agent
        .update(canister_id, "burn_nft")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    println!("NFT burned successfully");
    Ok(())
}

async fn nft_update_metadata(
    agent: &Agent,
    canister_id: &Principal,
    token_id: Nat,
    token_name: Option<String>,
    token_description: Option<String>,
    token_logo: Option<String>,
) -> Result<()> {
    println!("Updating NFT metadata for token ID: {}...", token_id);

    if let Some(name) = &token_name {
        println!("New name: {}", name);
    }
    if let Some(desc) = &token_description {
        println!("New description: {}", desc);
    }
    if let Some(logo) = &token_logo {
        println!("New logo: {}", logo);
    }

    let args = management::update_nft_metadata::Args {
        token_id: token_id.clone(),
        token_name,
        token_description,
        token_logo,
        token_metadata: None,
    };

    let bytes = Encode!(&args)?;
    let _ = agent
        .update(canister_id, "update_nft_metadata")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    println!("NFT metadata updated successfully");
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
                chunk_id: Nat::from(i),
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
    let matches = Command::new("ICRC7 NFT CLI Tool")
        .version("1.0")
        .author("Gautier Wojda <gautier.wojda@bity.com>")
        .about("Manage ICRC7 NFTs")
        .subcommand(
            Command::new("upload-file")
                .about("Upload a file to the canister")
                .arg(arg!(<canister_id> "Target canister ID"))
                .arg(arg!(<file_path> "Path to the file to upload"))
                .arg(arg!(<destination_path> "Destination path in the canister (e.g. /images/photo.jpg)"))
                .arg(arg!(<identity> "Path to the identity PEM file")),
        )
        .subcommand(
            Command::new("mint")
                .about("Mint a new NFT")
                .arg(arg!(<canister_id> "Target canister ID"))
                .arg(arg!(<token_name> "Name of the NFT"))
                .arg(arg!(<owner> "Principal ID of the NFT owner"))
                .arg(arg!(<identity> "Path to the identity PEM file"))
                .arg(arg!(--description <description> "Description of the NFT").required(false))
                .arg(arg!(--logo <logo> "Logo URL of the NFT").required(false)),
        )
        .subcommand(
            Command::new("burn")
                .about("Burn an existing NFT")
                .arg(arg!(<canister_id> "Target canister ID"))
                .arg(arg!(<token_id> "ID of the NFT to burn"))
                .arg(arg!(<identity> "Path to the identity PEM file")),
        )
        .subcommand(
            Command::new("update-metadata")
                .about("Update NFT metadata")
                .arg(arg!(<canister_id> "Target canister ID"))
                .arg(arg!(<token_id> "ID of the NFT to update"))
                .arg(arg!(<identity> "Path to the identity PEM file"))
                .arg(arg!(--name <name> "New name for the NFT").required(false))
                .arg(arg!(--description <description> "New description for the NFT").required(false))
                .arg(arg!(--logo <logo> "New logo URL for the NFT").required(false)),
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
        Some(("mint", sub_matches)) => {
            let canister_id_str = sub_matches.get_one::<String>("canister_id").unwrap();
            let token_name = sub_matches
                .get_one::<String>("token_name")
                .unwrap()
                .to_string();
            let owner_str = sub_matches.get_one::<String>("owner").unwrap();
            let identity_path = sub_matches.get_one::<String>("identity").unwrap();

            let description = sub_matches
                .get_one::<String>("description")
                .map(|s| s.to_string());
            let logo = sub_matches.get_one::<String>("logo").map(|s| s.to_string());

            let canister_id = Principal::from_text(canister_id_str)?;
            let owner_principal = Principal::from_text(owner_str)?;
            let agent = initialize_agent(identity_path).await?;

            nft_mint(
                &agent,
                &canister_id,
                token_name,
                description,
                logo,
                owner_principal,
            )
            .await?;
        }
        Some(("burn", sub_matches)) => {
            let canister_id_str = sub_matches.get_one::<String>("canister_id").unwrap();
            let token_id_str = sub_matches.get_one::<String>("token_id").unwrap();
            let identity_path = sub_matches.get_one::<String>("identity").unwrap();

            let canister_id = Principal::from_text(canister_id_str)?;
            let token_id = Nat::from_str(token_id_str).map_err(|_| anyhow!("Invalid token ID"))?;
            let agent = initialize_agent(identity_path).await?;

            nft_burn(&agent, &canister_id, token_id).await?;
        }
        Some(("update-metadata", sub_matches)) => {
            let canister_id_str = sub_matches.get_one::<String>("canister_id").unwrap();
            let token_id_str = sub_matches.get_one::<String>("token_id").unwrap();
            let identity_path = sub_matches.get_one::<String>("identity").unwrap();

            let name = sub_matches.get_one::<String>("name").map(|s| s.to_string());
            let description = sub_matches
                .get_one::<String>("description")
                .map(|s| s.to_string());
            let logo = sub_matches.get_one::<String>("logo").map(|s| s.to_string());

            // Require at least one of the optional parameters
            if name.is_none() && description.is_none() && logo.is_none() {
                return Err(anyhow!(
                    "At least one of --name, --description, or --logo must be provided"
                ));
            }

            let canister_id = Principal::from_text(canister_id_str)?;
            let token_id = Nat::from_str(token_id_str).map_err(|_| anyhow!("Invalid token ID"))?;
            let agent = initialize_agent(identity_path).await?;

            nft_update_metadata(&agent, &canister_id, token_id, name, description, logo).await?;
        }
        _ => {
            println!("Please specify a valid subcommand. Use --help for more information.");
            return Ok(());
        }
    }

    Ok(())
}
