use anyhow::{anyhow, Result};
use candid::{Encode, Nat, Principal};
use clap::{arg, value_parser, ArgAction, Command};
use core_nft::types::management::mint;
use core_nft::updates::management::{finalize_upload, init_upload, store_chunk};
use ic_agent::{identity::Secp256k1Identity, Agent};
use icrc_ledger_types::icrc1::account::Account;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{write, File};
use std::io::{Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;
use url::Url;

async fn initialize_agent(identity_pem_path: &str, network: &str) -> Result<Agent> {
    let identity = Secp256k1Identity::from_pem_file(identity_pem_path)?;

    let url = match network {
        "local" => "http://127.0.0.1:4943",
        "ic" => "https://ic0.app",
        custom => custom,
    };

    let agent = Agent::builder()
        .with_identity(identity)
        .with_url(url)
        .build()
        .expect("Failed to create Internet Computer agent. This should not happen.");

    if network != "ic" {
        agent.fetch_root_key().await?;
    }

    Ok(agent)
}

fn validate_icrc97_metadata(metadata: &Value) -> Result<()> {
    if !metadata.is_object() {
        return Err(anyhow!("Metadata must be a JSON object"));
    }

    let obj = metadata.as_object().unwrap();

    if let Some(name) = obj.get("name") {
        if !name.is_string() {
            return Err(anyhow!("'name' field must be a string"));
        }
    }

    if let Some(description) = obj.get("description") {
        if !description.is_string() {
            return Err(anyhow!("'description' field must be a string"));
        }
    }

    if let Some(image) = obj.get("image") {
        if !image.is_string() {
            return Err(anyhow!("'image' field must be a string"));
        }
    }

    if let Some(external_url) = obj.get("external_url") {
        if !external_url.is_string() {
            return Err(anyhow!("'external_url' field must be a string"));
        }
    }

    if let Some(attributes) = obj.get("attributes") {
        if !attributes.is_array() {
            return Err(anyhow!("'attributes' field must be an array"));
        }

        for (i, attr) in attributes.as_array().unwrap().iter().enumerate() {
            if !attr.is_object() {
                return Err(anyhow!("Attribute {} must be an object", i));
            }

            let attr_obj = attr.as_object().unwrap();

            if !attr_obj.contains_key("trait_type") {
                return Err(anyhow!("Attribute {} must have 'trait_type' field", i));
            }

            if !attr_obj.get("trait_type").unwrap().is_string() {
                return Err(anyhow!("Attribute {} 'trait_type' must be a string", i));
            }

            if !attr_obj.contains_key("value") {
                return Err(anyhow!("Attribute {} must have 'value' field", i));
            }

            if let Some(display_type) = attr_obj.get("display_type") {
                if !display_type.is_string() {
                    return Err(anyhow!("Attribute {} 'display_type' must be a string", i));
                }
            }
        }
    }

    Ok(())
}

fn prompt_input(prompt: &str) -> String {
    print!("{}: ", prompt);
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn prompt_optional(prompt: &str) -> Option<String> {
    let input = prompt_input(&format!("{} (optional, press Enter to skip)", prompt));
    if input.is_empty() {
        None
    } else {
        Some(input)
    }
}

fn prompt_bool(prompt: &str) -> bool {
    loop {
        let input = prompt_input(&format!("{} (y/n)", prompt));
        match input.to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => println!("Please enter 'y' or 'n'"),
        }
    }
}

fn prompt_display_type(value_is_number: bool) -> Option<String> {
    if value_is_number {
        println!("\nDisplay type options for numbers:");
        println!("  1. number - Regular number display");
        println!("  2. boost_number - Number with + prefix");
        println!("  3. boost_percentage - Percentage with + prefix");
        println!("  4. date - Unix timestamp as date");
        println!("  5. (none) - No special display");

        loop {
            let input = prompt_input("Choose display type (1-5)");
            match input.as_str() {
                "1" => return Some("number".to_string()),
                "2" => return Some("boost_number".to_string()),
                "3" => return Some("boost_percentage".to_string()),
                "4" => return Some("date".to_string()),
                "5" | "" => return None,
                _ => println!("Please enter a number between 1-5 or press Enter for none"),
            }
        }
    } else {
        println!("\nDisplay type options for text:");
        println!("  1. (none) - Default text display");
        println!("  2. custom - Enter custom display type");

        loop {
            let input = prompt_input("Choose display type (1-2)");
            match input.as_str() {
                "1" | "" => return None,
                "2" => {
                    let custom = prompt_input("Enter custom display type");
                    return if custom.is_empty() {
                        None
                    } else {
                        Some(custom)
                    };
                }
                _ => println!("Please enter 1, 2, or press Enter for none"),
            }
        }
    }
}

fn create_metadata_interactive() -> Result<Value> {
    println!("=== ICRC97 Metadata Creation ===");

    let name = prompt_input("NFT name");
    if name.is_empty() {
        return Err(anyhow!("Name is required"));
    }

    let description = prompt_input("NFT description");
    if description.is_empty() {
        return Err(anyhow!("Description is required"));
    }

    let image = prompt_optional("Image URL");
    let external_url = prompt_optional("External URL");

    let mut metadata = json!({
        "name": name,
        "description": description
    });

    if let Some(img) = image {
        metadata["image"] = json!(img);
    }

    if let Some(ext_url) = external_url {
        metadata["external_url"] = json!(ext_url);
    }

    if prompt_bool("Add attributes?") {
        let mut attributes = Vec::new();

        loop {
            println!("\n--- Adding attribute ---");
            let trait_type = prompt_input("Trait type");
            if trait_type.is_empty() {
                break;
            }

            let value_str = prompt_input("Value");
            let (value, is_number) = if let Ok(num) = value_str.parse::<f64>() {
                (json!(num), true)
            } else {
                (json!(value_str), false)
            };

            let display_type = prompt_display_type(is_number);

            let mut attr = json!({
                "trait_type": trait_type,
                "value": value
            });

            if let Some(display) = display_type {
                attr["display_type"] = json!(display);
            }

            attributes.push(attr);

            if !prompt_bool("Add another attribute?") {
                break;
            }
        }

        if !attributes.is_empty() {
            metadata["attributes"] = json!(attributes);
        }
    }

    Ok(metadata)
}

async fn nft_init_upload(
    agent: &Agent,
    canister_id: &Principal,
    args: init_upload::Args,
) -> Result<()> {
    println!("Initializing upload...");
    println!("File: {}", args.file_path);
    println!("Size: {} bytes", args.file_size);
    println!("SHA-256: {}", args.file_hash);
    if let Some(chunk_size) = args.chunk_size {
        println!("Chunk size: {} bytes", chunk_size);
    }

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
    let bytes = Encode!(&args)?;
    let _ = agent
        .update(canister_id, "store_chunk")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    Ok(())
}

async fn nft_finalize_upload(
    agent: &Agent,
    canister_id: &Principal,
    args: finalize_upload::Args,
) -> Result<Url> {
    println!("Finalizing upload...");

    let bytes = Encode!(&args)?;
    let response = agent
        .update(canister_id, "finalize_upload")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    let url = candid::decode_one::<finalize_upload::Response>(&response)?
        .map_err(|e| anyhow!("Finalize upload failed: {:?}", e))?;

    println!("Upload finalized successfully");
    Ok(Url::parse(&url.url)?)
}

async fn upload_file_to_canister(
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
    println!("Uploading {} chunks...", total_chunks);

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

        let progress = ((i + 1) as f64 / total_chunks as f64 * 100.0) as u8;
        print!("\rProgress: {}% ({}/{})", progress, i + 1, total_chunks);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }
    println!();

    let url = nft_finalize_upload(
        &agent,
        &canister_id,
        finalize_upload::Args {
            file_path: destination_path.to_string(),
        },
    )
    .await?;

    Ok(url)
}

fn create_icrc97_metadata(
    name: &str,
    description: &str,
    image_url: Option<&str>,
    external_url: Option<&str>,
    attributes: Vec<(String, Value, Option<String>)>,
) -> Value {
    let mut metadata = json!({
        "name": name,
        "description": description
    });

    if let Some(image) = image_url {
        metadata["image"] = json!(image);
    }

    if let Some(external) = external_url {
        metadata["external_url"] = json!(external);
    }

    if !attributes.is_empty() {
        let attrs: Vec<Value> = attributes
            .into_iter()
            .map(|(trait_type, value, display_type)| {
                let mut attr = json!({
                    "trait_type": trait_type,
                    "value": value
                });
                if let Some(display) = display_type {
                    attr["display_type"] = json!(display);
                }
                attr
            })
            .collect();
        metadata["attributes"] = json!(attrs);
    }

    metadata
}

async fn mint_nft(
    agent: &Agent,
    canister_id: &Principal,
    owner: Principal,
    subaccount: Option<[u8; 32]>,
    token_name: &str,
    metadata_url: &str,
    memo: Option<&str>,
) -> Result<Nat> {
    println!("Minting NFT...");
    println!("Owner: {}", owner);
    println!("Name: {}", token_name);
    println!("Metadata URL: {}", metadata_url);

    let mint_args = mint::Args {
        token_name: token_name.to_string(),
        token_metadata_url: metadata_url.to_string(),
        token_owner: Account { owner, subaccount },
        memo: memo.map(|m| serde_bytes::ByteBuf::from(m.as_bytes())),
    };

    let bytes = Encode!(&mint_args)?;
    let response = agent
        .update(canister_id, "mint")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    let token_id = candid::decode_one::<mint::Response>(&response)?
        .map_err(|e| anyhow!("Mint failed: {:?}", e))?;

    println!("NFT minted successfully with token ID: {}", token_id);
    Ok(token_id)
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("ICRC7 NFT Tool")
        .version("1.0")
        .author("Gautier Wojda <gautier.wojda@bity.com>")
        .about("Complete tool for ICRC7 NFT management")
        .arg(
            arg!(-n --network <NETWORK> "Network to connect to")
                .value_parser(["local", "ic"])
                .default_value("local")
        )
        .arg(
            arg!(-i --identity <IDENTITY> "Path to identity PEM file")
                .required(true)
        )
        .arg(
            arg!(-c --canister <CANISTER_ID> "Target canister ID")
                .required(true)
        )
        .subcommand(
            Command::new("upload-file")
                .about("Upload a file to the canister")
                .arg(arg!(<file_path> "Path to the file to upload"))
                .arg(arg!(<destination_path> "Destination path in the canister (e.g. /images/photo.jpg)"))
                .arg(
                    arg!(-s --chunk_size <SIZE> "Chunk size in bytes")
                        .value_parser(value_parser!(u64))
                        .default_value("1048576")
                )
        )
        .subcommand(
            Command::new("validate-metadata")
                .about("Validate ICRC97 JSON metadata file")
                .arg(arg!(<metadata_file> "Path to JSON metadata file"))
        )
        .subcommand(
            Command::new("create-metadata")
                .about("Create ICRC97 metadata interactively or from parameters")
                .arg(arg!(-o --output <FILE> "Output JSON file path").required(true))
                .arg(arg!(-i --interactive "Use interactive mode"))
                .arg(arg!(-n --name <NAME> "NFT name"))
                .arg(arg!(-d --description <DESC> "NFT description"))
                .arg(arg!(--image <URL> "Image URL"))
                .arg(arg!(--external_url <URL> "External URL"))
                .arg(
                    arg!(-a --attribute <ATTR> "Add attribute (format: trait_type:value[:display_type])")
                        .action(ArgAction::Append)
                )
        )
        .subcommand(
            Command::new("upload-metadata")
                .about("Upload JSON metadata file to canister")
                .arg(arg!(<metadata_file> "Path to JSON metadata file"))
                .arg(
                    arg!(-s --chunk_size <SIZE> "Chunk size in bytes")
                        .value_parser(value_parser!(u64))
                        .default_value("1048576")
                )
        )
        .subcommand(
            Command::new("mint")
                .about("Mint an NFT with metadata URL")
                .arg(arg!(-o --owner <OWNER> "Owner principal").required(true))
                .arg(arg!(-n --name <NAME> "Token name").required(true))
                .arg(arg!(-u --metadata_url <URL> "Metadata URL").required(true))
                .arg(arg!(--memo <MEMO> "Optional memo"))
                .arg(
                    arg!(--subaccount <SUBACCOUNT> "Optional subaccount (32 bytes hex)")
                )
        )
        .subcommand(
            Command::new("mint-with-metadata")
                .about("Create metadata and mint NFT")
                .arg(arg!(-o --owner <OWNER> "Owner principal").required(true))
                .arg(arg!(-n --name <NAME> "NFT name").required(true))
                .arg(arg!(--memo <MEMO> "Optional memo"))
                .arg(
                    arg!(--subaccount <SUBACCOUNT> "Optional subaccount (32 bytes hex)")
                )
                .arg(arg!(-f --file <FILE> "Use JSON metadata file"))
                .arg(arg!(--interactive "Use interactive mode"))
                .arg(arg!(-d --description <DESC> "NFT description (CLI mode)"))
                .arg(arg!(--image <URL> "Image URL (CLI mode)"))
                .arg(arg!(--external_url <URL> "External URL (CLI mode)"))
                .arg(
                    arg!(-a --attribute <ATTR> "Add attribute (CLI mode, format: trait_type:value[:display_type])")
                        .action(ArgAction::Append)
                )
        )
        .get_matches();

    let network = matches.get_one::<String>("network").unwrap();
    let identity_path = matches.get_one::<String>("identity").unwrap();
    let canister_id_str = matches.get_one::<String>("canister").unwrap();

    let canister_id = Principal::from_text(canister_id_str)?;
    let agent = initialize_agent(identity_path, network).await?;

    match matches.subcommand() {
        Some(("upload-file", sub_matches)) => {
            let file_path = sub_matches.get_one::<String>("file_path").unwrap();
            let destination_path = sub_matches.get_one::<String>("destination_path").unwrap();
            let chunk_size = *sub_matches.get_one::<u64>("chunk_size").unwrap();

            if !Path::new(file_path).exists() {
                return Err(anyhow!("File '{}' does not exist", file_path));
            }

            let url = upload_file_to_canister(
                &agent,
                &canister_id,
                file_path,
                destination_path,
                chunk_size,
            )
            .await?;

            println!("File upload completed successfully!");
            println!("URL: {}", url);
        }

        Some(("validate-metadata", sub_matches)) => {
            let metadata_file = sub_matches.get_one::<String>("metadata_file").unwrap();

            if !Path::new(metadata_file).exists() {
                return Err(anyhow!("Metadata file '{}' does not exist", metadata_file));
            }

            let metadata_content = std::fs::read_to_string(metadata_file)?;
            let metadata: Value = serde_json::from_str(&metadata_content)
                .map_err(|e| anyhow!("Invalid JSON: {}", e))?;

            validate_icrc97_metadata(&metadata)?;

            println!("Metadata validation successful!");
            println!("File: {}", metadata_file);
            println!("Content preview:");
            println!("{}", serde_json::to_string_pretty(&metadata)?);
        }

        Some(("create-metadata", sub_matches)) => {
            let output_file = sub_matches.get_one::<String>("output").unwrap();
            let interactive = sub_matches.get_flag("interactive");
            let name = sub_matches.get_one::<String>("name");
            let description = sub_matches.get_one::<String>("description");

            let metadata = if interactive || (name.is_none() && description.is_none()) {
                create_metadata_interactive()?
            } else {
                let name = name.ok_or_else(|| anyhow!("Name is required in non-interactive mode (use --interactive for interactive mode)"))?;
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
                                json!(num)
                            } else {
                                json!(parts[1])
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
        }

        Some(("upload-metadata", sub_matches)) => {
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

            let url = upload_file_to_canister(
                &agent,
                &canister_id,
                metadata_file,
                &destination_path,
                chunk_size,
            )
            .await?;

            println!("Metadata uploaded successfully!");
            println!("URL: {}", url);
        }

        Some(("mint", sub_matches)) => {
            let owner_str = sub_matches.get_one::<String>("owner").unwrap();
            let token_name = sub_matches.get_one::<String>("name").unwrap();
            let metadata_url = sub_matches.get_one::<String>("metadata_url").unwrap();
            let memo = sub_matches.get_one::<String>("memo");

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

            let token_id = mint_nft(
                &agent,
                &canister_id,
                owner,
                subaccount,
                token_name,
                metadata_url,
                memo.map(|s| s.as_str()),
            )
            .await?;

            println!("NFT minted successfully with ID: {}", token_id);
        }

        Some(("mint-with-metadata", sub_matches)) => {
            let owner_str = sub_matches.get_one::<String>("owner").unwrap();
            let name = sub_matches.get_one::<String>("name").unwrap();
            let memo = sub_matches.get_one::<String>("memo");

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

            let metadata = if let Some(file_path) = sub_matches.get_one::<String>("file") {
                if !Path::new(file_path).exists() {
                    return Err(anyhow!("Metadata file '{}' does not exist", file_path));
                }
                let content = std::fs::read_to_string(file_path)?;
                let metadata: Value = serde_json::from_str(&content)?;
                validate_icrc97_metadata(&metadata)?;
                println!("Using metadata from file: {}", file_path);
                metadata
            } else if sub_matches.get_flag("interactive") {
                create_metadata_interactive()?
            } else {
                let description = sub_matches
                    .get_one::<String>("description")
                    .ok_or_else(|| anyhow!("Description is required in CLI mode"))?;
                let image_url = sub_matches.get_one::<String>("image");
                let external_url = sub_matches.get_one::<String>("external_url");

                let mut attributes = Vec::new();
                if let Some(attrs) = sub_matches.get_many::<String>("attribute") {
                    for attr in attrs {
                        let parts: Vec<&str> = attr.split(':').collect();
                        if parts.len() >= 2 {
                            let trait_type = parts[0].to_string();
                            let value = if let Ok(num) = parts[1].parse::<f64>() {
                                json!(num)
                            } else {
                                json!(parts[1])
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

            let mut temp_file = NamedTempFile::new()?;
            let json_string = serde_json::to_string_pretty(&metadata)?;
            write(temp_file.path(), &json_string)?;

            let hash = Sha256::digest(json_string.as_bytes());
            let hash_string = format!("{:x}", hash);
            let destination_path = format!("/{}.json", hash_string);

            let metadata_url = upload_file_to_canister(
                &agent,
                &canister_id,
                temp_file.path().to_str().unwrap(),
                &destination_path,
                1048576,
            )
            .await?;

            println!("Metadata uploaded: {}", metadata_url);

            let token_id = mint_nft(
                &agent,
                &canister_id,
                owner,
                subaccount,
                name,
                &metadata_url.to_string(),
                memo.map(|s| s.as_str()),
            )
            .await?;

            println!("NFT created and minted successfully!");
            println!("Token ID: {}", token_id);
            println!("Metadata URL: {}", metadata_url);
        }

        _ => {
            println!("Please specify a valid subcommand. Use --help for more information.");
        }
    }

    Ok(())
}
