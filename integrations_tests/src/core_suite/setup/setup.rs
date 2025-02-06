use crate::core_suite::setup::setup_core::setup_core_canister;
use crate::utils::random_principal;
use core_nft::lifecycle::Args;
use core_nft::init::InitArgs;
use core_nft::post_upgrade::UpgradeArgs;
use candid::CandidType;
use candid::Deserialize;
use candid::Principal;
use ic_ledger_types::Tokens;
use pocket_ic::{ PocketIc, PocketIcBuilder };
use types::BuildVersion;
use types::CanisterId;
use types::TokenInfo;

#[derive(CandidType, Deserialize, Debug)]
pub struct RegisterDappCanisterRequest {
    pub canister_id: Option<Principal>,
}

pub struct TestEnv {
    pub controller: Principal,
    pub buyback_burn_id: CanisterId,
    pub pic: PocketIc,
}

use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
impl Debug for TestEnv {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestEnv").field("buyback_burn_id", &self.buyback_burn_id.to_text()).finish()
    }
}
pub struct TestEnvBuilder {
    controller: Principal,
    collection_id: CanisterId,
}

impl Default for TestEnvBuilder {
    fn default() -> Self {
        Self {
            controller: random_principal(),
            collection_id: Principal::from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        }
    }
}

impl TestEnvBuilder {
    pub fn new() -> Self {
        TestEnvBuilder::default()
    }

    pub fn with_controller(mut self, principal: Principal) -> Self {
        self.controller = principal;
        self
    }

    pub fn build(&mut self) -> TestEnv {
        let mut pic = PocketIcBuilder::new().with_application_subnet().build();

        self.collection_id = pic.create_canister_with_settings(Some(self.controller.clone()), None);

        let nft_init_args = Args::Init(InitArgs {
            test_mode: true,
            version: BuildVersion::min(),
            commit_hash: "commit_hash".to_string(),
            authorized_principals: vec![self.controller.clone()],
            minting_authorities: vec![self.controller.clone()],
            description: None,
            symbol: "symbol".to_string(),
            name: "name".to_string(),
            logo: None,
            supply_cap: None,
            max_query_batch_size: None,
            max_update_batch_size: None,
            max_take_value: None,
            default_take_value: None,
            max_memo_size: None,
            atomic_batch_transfers: None,
            tx_window: None,
            permitted_drift: None,
            max_canister_storage_threshold: None,
            collection_metadata: HashMap::new(),
            approval_init: None,
        });

        let buyback_burn_canister_id = setup_core_canister(
            &mut pic,
            self.collection_id,
            nft_init_args,
            self.controller
        );

        TestEnv {
            controller: self.controller,
            buyback_burn_id: buyback_burn_canister_id,
            pic,
        }
    }
}
