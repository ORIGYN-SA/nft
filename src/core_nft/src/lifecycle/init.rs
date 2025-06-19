use std::collections::HashMap;
use std::time::Duration;

use crate::lifecycle::init_canister;
use crate::lifecycle::Args;
pub use crate::state::InitApprovalsArg;
use crate::state::{init_icrc3, start_default_archive_job, Data, RuntimeState};
use crate::types::http::certify_all_assets;
use crate::types::value_custom::CustomValue as Value;

use bity_ic_canister_tracing_macros::trace;
use bity_ic_icrc3::config::{ICRC3Config, ICRC3Properties};
use bity_ic_types::BuildVersion;
use bity_ic_utils::env::{CanisterEnv, Environment};
use candid::Principal;
use candid::{CandidType, Nat};
use ic_cdk_macros::init;
use icrc_ledger_types::icrc3::blocks::SupportedBlockType;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct InitArgs {
    pub test_mode: bool,
    pub version: BuildVersion,
    pub commit_hash: String,
    pub authorized_principals: Vec<Principal>,
    pub minting_authorities: Vec<Principal>,
    pub description: Option<String>,
    pub symbol: String,
    pub name: String,
    pub logo: Option<String>,
    pub supply_cap: Option<Nat>,
    pub max_query_batch_size: Option<Nat>,
    pub max_update_batch_size: Option<Nat>,
    pub max_take_value: Option<Nat>,
    pub default_take_value: Option<Nat>,
    pub max_memo_size: Option<Nat>,
    pub atomic_batch_transfers: Option<bool>,
    pub tx_window: Option<Nat>,
    pub permitted_drift: Option<Nat>,
    pub max_canister_storage_threshold: Option<Nat>,
    pub collection_metadata: HashMap<String, Value>,
    pub approval_init: InitApprovalsArg,
}

#[init]
#[trace]
fn init(args: Args) {
    match args {
        Args::Init(init_args) => {
            info!("Init start.");
            let env = CanisterEnv::new(
                init_args.test_mode,
                init_args.version,
                init_args.commit_hash.clone(),
            );

            match init_args.logo.clone() {
                Some(logo) => {
                    let logo_url = logo.clone();
                    if let Err(_) = url::Url::parse(&logo_url) {
                        ic_cdk::trap(&format!("Invalid logo URL: {}", logo_url));
                    }
                }
                None => {}
            };

            let mut data = Data::new(
                init_args.test_mode,
                init_args.commit_hash,
                init_args.version,
                init_args.authorized_principals,
                init_args.minting_authorities,
                init_args.description,
                init_args.symbol,
                init_args.name,
                init_args.logo,
                init_args.supply_cap,
                init_args.max_query_batch_size,
                init_args.max_update_batch_size,
                init_args.max_take_value,
                init_args.default_take_value,
                init_args.max_memo_size,
                init_args.atomic_batch_transfers,
                init_args.tx_window.clone(),
                init_args.permitted_drift,
                init_args.max_canister_storage_threshold,
                init_args.approval_init,
            );

            if env.is_test_mode() {
                data.authorized_principals.push(env.caller());
            }

            let _tx_window = match init_args.tx_window {
                Some(tx_window) => Duration::from_millis(u64::try_from(tx_window.0).unwrap()),
                None => Duration::from_secs(0),
            };

            let icrc3_config = ICRC3Config {
                supported_blocks: vec![SupportedBlockType {
                    block_type: "7mint".to_string(),
                    url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-7/ICRC-7.md#mint-block-schema".to_string(),
                },
                SupportedBlockType {
                    block_type: "7burn".to_string(),
                    url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-7/ICRC-7.md#burn-block-schema".to_string(),
                },
                SupportedBlockType {
                    block_type: "7xfer".to_string(),
                    url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-7/ICRC-7.md#icrc7_transfer-block-schema".to_string(),
                },
                SupportedBlockType {
                    block_type: "7update_token".to_string(),
                    url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-7/ICRC-7.md#update-token-block-schema".to_string(),
                },
                SupportedBlockType {
                    block_type: "37approve".to_string(),
                    url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-37/ICRC-37.md#icrc37_approve_tokens-block-schema".to_string(),
                },
                SupportedBlockType {
                    block_type: "37approve_coll".to_string(),
                    url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-37/ICRC-37.md#icrc37_approve_collection-block-schema".to_string(),
                },
                SupportedBlockType {
                    block_type: "37revoke".to_string(),
                    url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-37/ICRC-37.md#icrc37_revoke_token_approvals-block-schema".to_string(),
                },
                SupportedBlockType {
                    block_type: "37revoke_coll".to_string(),
                    url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-37/ICRC-37.md#icrc37_revoke_collection_approvals-block-schema".to_string(),
                },
                SupportedBlockType {
                    block_type: "37xfer".to_string(),
                    url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-37/ICRC-37.md#icrc37_transfer_from-block-schema".to_string(),
                }
                ],
                constants: ICRC3Properties::new(
                    _tx_window,
                    10,
                    6 * 1024 * 1024,
                    2 * 1024 * 1024,
                    2_000_000_000_000_000_000,
                    2_000_000_000_000_000_000,
                    25,
                    Duration::from_secs(120),
                    1000
                ),
            };

            let runtime_state = RuntimeState::new(env, data);

            init_canister(runtime_state);
            init_icrc3(icrc3_config);
            start_default_archive_job();
            certify_all_assets();

            info!("Init complete.")
        }
        Args::Upgrade(_) => {
            panic!(
                "Cannot initialize the canister with an Upgrade argument. Please provide an Init argument."
            );
        }
    }
}
