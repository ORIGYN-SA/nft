use crate::state::read_state;
pub use core_nft_api::queries::get_private_content_metadata::Args as GetPrivateContentMetadataArgs;
pub use core_nft_api::queries::get_private_content_metadata::Response as GetPrivateContentMetadataResponse;
pub use core_nft_api::queries::get_public_content_metadata::Args as GetPublicContentMetadataArgs;
pub use core_nft_api::queries::get_public_content_metadata::Response as GetPublicContentMetadataResponse;
use core_nft_common::types::private_content::EntryDetailResp as PrivateEntryDetailResp;
use core_nft_common::types::public_content::EntryDetailResp as PublicEntryDetailResp;
use ic_cdk_macros::query;

#[query]
pub fn get_public_content_metadata(
    args: GetPublicContentMetadataArgs,
) -> GetPublicContentMetadataResponse {
    read_state(|state| {
        let record = state
            .data
            .public_content_system
            .nft_public
            .get(&args.token_id)
            .ok_or_else(|| format!("NFT {} has no public content", args.token_id))?;

        let entries: Vec<PublicEntryDetailResp> = match &args.entry_name {
            Some(name) => {
                let entry = record
                    .entries
                    .get(name)
                    .ok_or_else(|| format!("Public entry '{}' not found", name))?;
                vec![PublicEntryDetailResp {
                    name: name.clone(),
                    state: entry.state.clone(),
                    hash: entry.hash.clone(),
                    file_size: entry.file_size,
                    storage_canister_id: entry.storage_canister_id,
                    storage_path: entry.storage_path.clone(),
                }]
            }
            None => record
                .entries
                .iter()
                .map(|(name, entry)| PublicEntryDetailResp {
                    name: name.clone(),
                    state: entry.state.clone(),
                    hash: entry.hash.clone(),
                    file_size: entry.file_size,
                    storage_canister_id: entry.storage_canister_id,
                    storage_path: entry.storage_path.clone(),
                })
                .collect(),
        };

        Ok(entries)
    })
}

#[query]
pub fn get_private_content_metadata(
    args: GetPrivateContentMetadataArgs,
) -> GetPrivateContentMetadataResponse {
    read_state(|state| {
        let record = state
            .data
            .private_content_system
            .nft_private
            .get(&args.token_id)
            .ok_or_else(|| format!("NFT {} has no private content", args.token_id))?;

        let entries: Vec<PrivateEntryDetailResp> = match &args.entry_name {
            Some(name) => {
                let entry = record
                    .entries
                    .get(name)
                    .ok_or_else(|| format!("Private entry '{}' not found", name))?;

                let readers = entry
                    .readers
                    .iter()
                    .map(|(principal, info)| {
                        core_nft_common::types::private_content::ReaderDetail {
                            principal: *principal,
                            rights: info.rights.clone(),
                            alias: info.alias.clone(),
                        }
                    })
                    .collect();

                vec![PrivateEntryDetailResp {
                    name: name.clone(),
                    status: entry.status.clone(),
                    readers,
                    plaintext_hash: entry.hash,
                    plaintext_size: entry.plaintext_size,
                    storage_canister_id: entry.storage_canister_id,
                    storage_path: entry.storage_path.clone(),
                }]
            }
            None => record
                .entries
                .iter()
                .map(|(name, entry)| {
                    let readers = entry
                        .readers
                        .iter()
                        .map(|(principal, info)| {
                            core_nft_common::types::private_content::ReaderDetail {
                                principal: *principal,
                                rights: info.rights.clone(),
                                alias: info.alias.clone(),
                            }
                        })
                        .collect();

                    PrivateEntryDetailResp {
                        name: name.clone(),
                        status: entry.status.clone(),
                        readers,
                        plaintext_hash: entry.hash,
                        plaintext_size: entry.plaintext_size,
                        storage_canister_id: entry.storage_canister_id,
                        storage_path: entry.storage_path.clone(),
                    }
                })
                .collect(),
        };

        Ok(entries)
    })
}
