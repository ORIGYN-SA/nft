use anyhow::{anyhow, Result};
use candid::{Encode, Principal};
use core_nft::updates::management::{get_user_permissions, grant_permission, has_permission, revoke_permission, Permission};
use ic_agent::Agent;

pub fn parse_permission_from_str(input: &str) -> Result<Permission> {
    match input.to_lowercase().as_str() {
        "minting" => Ok(Permission::Minting),
        "manageauthorities" | "manage_authorities" | "manage-authorities" => {
            Ok(Permission::ManageAuthorities)
        }
        "updatemetadata" | "update_metadata" | "update-metadata" => {
            Ok(Permission::UpdateMetadata)
        }
        "updatecollectionmetadata"
        | "update_collection_metadata"
        | "update-collection-metadata" => Ok(Permission::UpdateCollectionMetadata),
        "readuploads" | "read_uploads" | "read-uploads" => Ok(Permission::ReadUploads),
        "updateuploads" | "update_uploads" | "update-uploads" => Ok(Permission::UpdateUploads),
        other => Err(anyhow!(
            "Unknown permission: {}. Valid values: minting, manage_authorities, update_metadata, update_collection_metadata, read_uploads, update_uploads",
            other
        )),
    }
}

pub async fn grant(
    agent: &Agent,
    canister_id: &Principal,
    principal: Principal,
    permission: Permission,
) -> Result<()> {
    let args = grant_permission::Args { principal, permission };
    let bytes = Encode!(&args)?;
    let response = agent
        .update(canister_id, "grant_permission")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    candid::decode_one::<grant_permission::Response>(&response)?
        .map_err(|e| anyhow!("Grant permission failed: {:?}", e))?;
    Ok(())
}

pub async fn revoke(
    agent: &Agent,
    canister_id: &Principal,
    principal: Principal,
    permission: Permission,
) -> Result<()> {
    let args = revoke_permission::Args { principal, permission };
    let bytes = Encode!(&args)?;
    let response = agent
        .update(canister_id, "revoke_permission")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    candid::decode_one::<revoke_permission::Response>(&response)?
        .map_err(|e| anyhow!("Revoke permission failed: {:?}", e))?;
    Ok(())
}

pub async fn list(
    agent: &Agent,
    canister_id: &Principal,
    principal: Principal,
) -> Result<Vec<Permission>> {
    let args = get_user_permissions::Args { principal };
    let bytes = Encode!(&args)?;
    let response = agent
        .query(canister_id, "get_user_permissions")
        .with_arg(bytes)
        .call()
        .await?;

    let permissions = candid::decode_one::<get_user_permissions::Response>(&response)?
        .map_err(|e| anyhow!("Fetching permissions failed: {:?}", e))?;
    Ok(permissions)
}

pub async fn has(
    agent: &Agent,
    canister_id: &Principal,
    principal: Principal,
    permission: Permission,
) -> Result<bool> {
    let args = has_permission::Args { principal, permission };
    let bytes = Encode!(&args)?;
    let response = agent
        .query(canister_id, "has_permission")
        .with_arg(bytes)
        .call()
        .await?;

    let has = candid::decode_one::<has_permission::Response>(&response)?
        .map_err(|e| anyhow!("Permission check failed: {:?}", e))?;
    Ok(has)
}


