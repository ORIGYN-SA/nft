use anyhow::Result;
use candid::Principal;
use clap::ArgMatches;

mod calls;
mod cli;
mod commands;
mod metadata;
mod prompts;
mod utils;

use cli::build_cli;
use commands::{
    handle_create_metadata, handle_mint, handle_permissions, handle_upload_file,
    handle_upload_metadata, handle_validate_metadata,
};
use utils::initialize_agent;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = build_cli().get_matches();

    let network = matches.get_one::<String>("network").unwrap();
    let identity_path = matches.get_one::<String>("identity").unwrap();
    let canister_id_str = matches.get_one::<String>("canister").unwrap();

    let canister_id = Principal::from_text(canister_id_str)?;
    let agent = initialize_agent(identity_path, network).await?;

    match matches.subcommand() {
        Some(("upload-file", sub_matches)) => {
            handle_upload_file(&agent, &canister_id, sub_matches).await?;
        }

        Some(("validate-metadata", sub_matches)) => {
            handle_validate_metadata(sub_matches).await?;
        }

        Some(("create-metadata", sub_matches)) => {
            handle_create_metadata(sub_matches).await?;
        }

        Some(("upload-metadata", sub_matches)) => {
            handle_upload_metadata(&agent, &canister_id, sub_matches).await?;
        }

        Some(("mint", sub_matches)) => {
            handle_mint(&agent, &canister_id, sub_matches).await?;
        }

        Some(("permissions", sub_matches)) => {
            handle_permissions(&agent, &canister_id, sub_matches).await?;
        }

        _ => {
            println!("Please specify a valid subcommand. Use --help for more information.");
        }
    }

    Ok(())
}
