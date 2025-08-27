use anyhow::{anyhow, Result};
use candid::Principal;
use clap::ArgMatches;
use ic_agent::Agent;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::write;
use std::path::Path;

use crate::calls::{
    mint as calls_mint, permissions as calls_permissions, uploads as calls_uploads,
};
use crate::metadata::{
    create_icrc97_metadata, create_icrc97_metadata_from_url, create_metadata_interactive,
    create_metadata_interactive_hashmap, validate_icrc97_metadata,
};
use core_nft::updates::management::Permission;

pub async fn handle_upload_file(
    agent: &Agent,
    canister_id: &Principal,
    sub_matches: &ArgMatches,
) -> Result<()> {
    let file_path = sub_matches.get_one::<String>("file_path").unwrap();
    let destination_path = sub_matches.get_one::<String>("destination_path").unwrap();
    let chunk_size = *sub_matches.get_one::<u64>("chunk_size").unwrap();

    if !Path::new(file_path).exists() {
        return Err(anyhow!("File '{}' does not exist", file_path));
    }

    let url =
        calls_uploads::upload_file(agent, canister_id, file_path, destination_path, chunk_size)
            .await?;

    println!("File upload completed successfully!");
    println!("URL: {}", url);
    Ok(())
}

pub async fn handle_validate_metadata(sub_matches: &ArgMatches) -> Result<()> {
    let metadata_file = sub_matches.get_one::<String>("metadata_file").unwrap();

    if !Path::new(metadata_file).exists() {
        return Err(anyhow!("Metadata file '{}' does not exist", metadata_file));
    }

    let metadata_content = std::fs::read_to_string(metadata_file)?;
    let metadata: Value =
        serde_json::from_str(&metadata_content).map_err(|e| anyhow!("Invalid JSON: {}", e))?;

    validate_icrc97_metadata(&metadata)?;

    println!("Metadata validation successful!");
    println!("File: {}", metadata_file);
    println!("Content preview:");
    println!("{}", serde_json::to_string_pretty(&metadata)?);
    Ok(())
}

pub async fn handle_create_metadata(sub_matches: &ArgMatches) -> Result<()> {
    let output_file = sub_matches.get_one::<String>("output").unwrap();
    let interactive = sub_matches.get_flag("interactive");
    let name = sub_matches.get_one::<String>("name");
    let description = sub_matches.get_one::<String>("description");

    let metadata = if interactive || (name.is_none() && description.is_none()) {
        create_metadata_interactive()?
    } else {
        let name = name.ok_or_else(|| {
            anyhow!(
                "Name is required in non-interactive mode (use --interactive for interactive mode)"
            )
        })?;
        let description = description.ok_or_else(|| anyhow!("Description is required in non-interactive mode (use --interactive for interactive mode)"))?;
        let image_url = sub_matches.get_one::<String>("image");
        let external_url = sub_matches.get_one::<String>("external_url");

        let mut attributes = Vec::new();
        if let Some(attrs) = sub_matches.get_many::<String>("attribute") {
            for attr in attrs {
                let parts: Vec<&str> = attr.split(':').collect();
                if parts.len() >= 2 {
                    let trait_type = parts[0].to_string();
                    let value = if let Ok(num) = parts[1].parse::<f64>() {
                        serde_json::json!(num)
                    } else {
                        serde_json::json!(parts[1])
                    };
                    let display_type = if parts.len() > 2 {
                        Some(parts[2].to_string())
                    } else {
                        None
                    };
                    attributes.push((trait_type, value, display_type));
                }
            }
        }

        create_icrc97_metadata(
            name,
            description,
            image_url.map(|s| s.as_str()),
            external_url.map(|s| s.as_str()),
            attributes,
        )
    };

    validate_icrc97_metadata(&metadata)?;

    let json_string = serde_json::to_string_pretty(&metadata)?;
    write(output_file, json_string)?;

    println!("Metadata created successfully: {}", output_file);
    println!("Content: {}", serde_json::to_string_pretty(&metadata)?);
    Ok(())
}

pub async fn handle_upload_metadata(
    agent: &Agent,
    canister_id: &Principal,
    sub_matches: &ArgMatches,
) -> Result<()> {
    let metadata_file = sub_matches.get_one::<String>("metadata_file").unwrap();
    let chunk_size = *sub_matches.get_one::<u64>("chunk_size").unwrap();

    if !Path::new(metadata_file).exists() {
        return Err(anyhow!("Metadata file '{}' does not exist", metadata_file));
    }

    let metadata_content = std::fs::read_to_string(metadata_file)?;
    let metadata: Value = serde_json::from_str(&metadata_content)?;
    validate_icrc97_metadata(&metadata)?;

    let hash = Sha256::digest(metadata_content.as_bytes());
    let hash_string = format!("{:x}", hash);
    let destination_path = format!("/{}.json", hash_string);

    let url = calls_uploads::upload_file(
        agent,
        canister_id,
        metadata_file,
        &destination_path,
        chunk_size,
    )
    .await?;

    println!("Metadata uploaded successfully!");
    println!("URL: {}", url);
    Ok(())
}

pub async fn handle_mint(
    agent: &Agent,
    canister_id: &Principal,
    sub_matches: &ArgMatches,
) -> Result<()> {
    let owner_str = sub_matches.get_one::<String>("owner").unwrap();
    let _token_name = sub_matches.get_one::<String>("name").unwrap();
    let memo = sub_matches.get_one::<String>("memo");
    let icrc97_url = sub_matches.get_one::<String>("icrc97_url");
    let interactive = sub_matches.get_flag("interactive");

    let owner = Principal::from_text(owner_str)?;
    let subaccount = if let Some(sub_str) = sub_matches.get_one::<String>("subaccount") {
        let bytes = hex::decode(sub_str)?;
        if bytes.len() != 32 {
            return Err(anyhow!("Subaccount must be exactly 32 bytes"));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Some(array)
    } else {
        None
    };

    let metadata = if let Some(url) = icrc97_url {
        println!("Creating ICRC97 metadata from URL: {}", url);
        create_icrc97_metadata_from_url(url)
    } else if interactive {
        println!("Using interactive mode to create metadata");
        create_metadata_interactive_hashmap()?
    } else {
        let mut metadata = Vec::new();
        if let Some(entries) = sub_matches.get_many::<String>("metadata") {
            for entry in entries {
                let parts: Vec<&str> = entry.split(':').collect();
                if parts.len() >= 2 {
                    let key = parts[0].to_string();
                    let value_str = parts[1];
                    let value = if let Ok(num) = value_str.parse::<u64>() {
                        icrc_ledger_types::icrc::generic_value::ICRC3Value::Nat(candid::Nat::from(
                            num,
                        ))
                    } else if let Ok(int) = value_str.parse::<i64>() {
                        icrc_ledger_types::icrc::generic_value::ICRC3Value::Int(candid::Int::from(
                            int,
                        ))
                    } else {
                        icrc_ledger_types::icrc::generic_value::ICRC3Value::Text(
                            value_str.to_string(),
                        )
                    };
                    metadata.push((key, value));
                }
            }
        }
        metadata
    };

    let token_id = calls_mint::mint_nft(
        agent,
        canister_id,
        owner,
        subaccount,
        metadata,
        memo.map(|s| s.as_str()),
    )
    .await?;

    println!("NFT minted successfully with ID: {}", token_id);
    Ok(())
}

pub async fn handle_permissions(
    agent: &Agent,
    canister_id: &Principal,
    sub_matches: &ArgMatches,
) -> Result<()> {
    match sub_matches.subcommand() {
        Some(("grant", sm)) => {
            let principal_text = sm.get_one::<String>("principal").unwrap();
            let perm_text = sm.get_one::<String>("permission").unwrap();
            let target = Principal::from_text(principal_text)?;
            let perm = calls_permissions::parse_permission_from_str(perm_text)?;
            calls_permissions::grant(agent, canister_id, target, perm).await?;
            println!("Permission granted successfully");
        }
        Some(("revoke", sm)) => {
            let principal_text = sm.get_one::<String>("principal").unwrap();
            let perm_text = sm.get_one::<String>("permission").unwrap();
            let target = Principal::from_text(principal_text)?;
            let perm = calls_permissions::parse_permission_from_str(perm_text)?;
            calls_permissions::revoke(agent, canister_id, target, perm).await?;
            println!("Permission revoked successfully");
        }
        Some(("list", sm)) => {
            let principal_text = sm.get_one::<String>("principal").unwrap();
            let target = Principal::from_text(principal_text)?;
            let permissions = calls_permissions::list(agent, canister_id, target).await?;
            if permissions.is_empty() {
                println!("No permissions");
            } else {
                println!("Permissions:");
                for p in permissions {
                    let label = match p {
                        Permission::Minting => "minting",
                        Permission::ManageAuthorities => "manage_authorities",
                        Permission::UpdateMetadata => "update_metadata",
                        Permission::UpdateCollectionMetadata => "update_collection_metadata",
                        Permission::ReadUploads => "read_uploads",
                        Permission::UpdateUploads => "update_uploads",
                    };
                    println!("- {}", label);
                }
            }
        }
        Some(("has", sm)) => {
            let principal_text = sm.get_one::<String>("principal").unwrap();
            let perm_text = sm.get_one::<String>("permission").unwrap();
            let target = Principal::from_text(principal_text)?;
            let perm = calls_permissions::parse_permission_from_str(perm_text)?;
            let has = calls_permissions::has(agent, canister_id, target, perm).await?;
            println!("{}", if has { "true" } else { "false" });
        }
        _ => {
            println!("Unknown permissions subcommand. Use --help.");
        }
    }
    Ok(())
}
