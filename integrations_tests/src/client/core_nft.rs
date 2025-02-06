use crate::{ generate_pocket_query_call, generate_pocket_update_call };

use core_nft::types::icrc7::{
    icrc7_collection_metadata,
    icrc7_symbol,
    icrc7_name,
    icrc7_description,
    icrc7_logo,
    icrc7_total_supply,
    icrc7_supply_cap,
    icrc7_max_query_batch_size,
    icrc7_max_update_batch_size,
    icrc7_default_take_value,
    icrc7_max_take_value,
    icrc7_max_memo_size,
    icrc7_atomic_batch_transfers,
    icrc7_tx_window,
    icrc7_permitted_drift,
    icrc7_token_metadata,
    icrc7_owner_of,
    icrc7_balance_of,
    icrc7_tokens,
    icrc7_tokens_of,
    icrc7_transfer,
};
use core_nft::types::management::{ mint, update_minting_authorities, update_nft_metadata };

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
generate_pocket_query_call!(icrc7_token_metadata);
generate_pocket_query_call!(icrc7_owner_of);
generate_pocket_query_call!(icrc7_balance_of);
generate_pocket_query_call!(icrc7_tokens);
generate_pocket_query_call!(icrc7_tokens_of);

generate_pocket_update_call!(icrc7_transfer);

generate_pocket_update_call!(mint);
generate_pocket_update_call!(update_nft_metadata);
generate_pocket_update_call!(update_minting_authorities);
