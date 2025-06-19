use crate::{generate_pocket_query_call, generate_pocket_update_call};

use bity_ic_storage_canister_api::{cancel_upload, finalize_upload, init_upload, store_chunk};

use core_nft::types::icrc3::{
    icrc3_get_archives, icrc3_get_blocks, icrc3_get_properties, icrc3_get_tip_certificate,
    icrc3_supported_block_types,
};
use core_nft::types::icrc37::{
    icrc37_approve_collection, icrc37_approve_tokens, icrc37_get_collection_approvals,
    icrc37_get_token_approvals, icrc37_is_approved, icrc37_max_approvals_per_token_or_collection,
    icrc37_max_revoke_approvals, icrc37_revoke_collection_approvals, icrc37_revoke_token_approvals,
    icrc37_transfer_from,
};
use core_nft::types::icrc7::{
    icrc7_atomic_batch_transfers, icrc7_balance_of, icrc7_collection_metadata,
    icrc7_default_take_value, icrc7_description, icrc7_logo, icrc7_max_memo_size,
    icrc7_max_query_batch_size, icrc7_max_take_value, icrc7_max_update_batch_size, icrc7_name,
    icrc7_owner_of, icrc7_permitted_drift, icrc7_supply_cap, icrc7_symbol, icrc7_token_metadata,
    icrc7_tokens, icrc7_tokens_of, icrc7_total_supply, icrc7_transfer, icrc7_tx_window,
};
use core_nft::types::management::{
    get_all_uploads, get_upload_status, mint, remove_authorized_principals,
    remove_minting_authorities, update_authorized_principals, update_collection_metadata,
    update_minting_authorities, update_nft_metadata,
};

generate_pocket_query_call!(icrc7_collection_metadata);
generate_pocket_query_call!(icrc7_symbol);
generate_pocket_query_call!(icrc7_name);
generate_pocket_query_call!(icrc7_description);
generate_pocket_query_call!(icrc7_logo);
generate_pocket_query_call!(icrc7_total_supply);
generate_pocket_query_call!(icrc7_supply_cap);
generate_pocket_query_call!(icrc7_max_query_batch_size);
generate_pocket_query_call!(icrc7_max_update_batch_size);
generate_pocket_query_call!(icrc7_default_take_value);
generate_pocket_query_call!(icrc7_max_take_value);
generate_pocket_query_call!(icrc7_max_memo_size);
generate_pocket_query_call!(icrc7_atomic_batch_transfers);
generate_pocket_query_call!(icrc7_tx_window);
generate_pocket_query_call!(icrc7_permitted_drift);
generate_pocket_query_call!(icrc7_owner_of);
generate_pocket_query_call!(icrc7_balance_of);
generate_pocket_query_call!(icrc7_token_metadata);
generate_pocket_query_call!(icrc3_get_archives);
generate_pocket_query_call!(icrc3_get_blocks);
generate_pocket_query_call!(icrc3_get_properties);
generate_pocket_query_call!(icrc3_get_tip_certificate);
generate_pocket_query_call!(icrc3_supported_block_types);

generate_pocket_query_call!(icrc37_is_approved);
generate_pocket_query_call!(icrc37_max_approvals_per_token_or_collection);
generate_pocket_query_call!(icrc37_max_revoke_approvals);

generate_pocket_update_call!(icrc7_transfer);

generate_pocket_update_call!(mint);
generate_pocket_update_call!(update_nft_metadata);
generate_pocket_update_call!(update_minting_authorities);
generate_pocket_update_call!(remove_minting_authorities);
generate_pocket_update_call!(update_authorized_principals);
generate_pocket_update_call!(remove_authorized_principals);
generate_pocket_update_call!(init_upload);
generate_pocket_update_call!(store_chunk);
generate_pocket_update_call!(finalize_upload);
generate_pocket_update_call!(cancel_upload);
generate_pocket_update_call!(update_collection_metadata);

generate_pocket_query_call!(get_upload_status);

generate_pocket_update_call!(icrc37_approve_collection);
generate_pocket_update_call!(icrc37_approve_tokens);
generate_pocket_update_call!(icrc37_revoke_collection_approvals);
generate_pocket_update_call!(icrc37_revoke_token_approvals);
generate_pocket_update_call!(icrc37_transfer_from);
