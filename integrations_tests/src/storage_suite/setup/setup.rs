use crate::storage_suite::setup::setup_storage::setup_storage_canister;
use crate::utils::random_principal;
use candid::CandidType;
use candid::Deserialize;
use candid::Principal;
use storage_api_canister::init::InitArgs;
use storage_api_canister::lifecycle::Args;
use pocket_ic::{ PocketIc, PocketIcBuilder };
use types::BuildVersion;
use types::CanisterId;

use std::time::Duration;

use types::Milliseconds;

pub const SECOND_IN_MS: Milliseconds = 1000;
pub const MINUTE_IN_MS: Milliseconds = SECOND_IN_MS * 60;
pub const HOUR_IN_MS: Milliseconds = MINUTE_IN_MS * 60;
pub const DAY_IN_MS: Milliseconds = HOUR_IN_MS * 24;

#[derive(CandidType, Deserialize, Debug)]
pub struct RegisterDappCanisterRequest {
    pub canister_id: Option<Principal>,
}

pub struct TestEnv {
    pub pic: PocketIc,
    pub storage_canister_id: CanisterId,
    pub controller: Principal,
    pub nft_owner1: Principal,
    pub nft_owner2: Principal,
}

use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
impl Debug for TestEnv {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestEnv")
            .field("buyback_burn_id", &self.storage_canister_id.to_text())
            .finish()
    }
}
pub struct TestEnvBuilder {
    controller: Principal,
    nft_owner1: Principal,
    nft_owner2: Principal,
    collection_id: CanisterId,
}

impl Default for TestEnvBuilder {
    fn default() -> Self {
        Self {
            controller: random_principal(),
            nft_owner1: random_principal(),
            nft_owner2: random_principal(),
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
        println!("Start building TestEnv");

        let mut pic = PocketIcBuilder::new()
            .with_application_subnet()
            .with_application_subnet()
            .with_sns_subnet()
            .with_fiduciary_subnet()
            .with_nns_subnet()
            .with_system_subnet()
            .build();

        self.collection_id = pic.create_canister_with_settings(Some(self.controller.clone()), None);

        pic.tick();
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 10));

        println!("collection_id: {}", self.collection_id.to_text());

        let storage_init_args = Args::Init(InitArgs {
            test_mode: true,
            version: BuildVersion::min(),
            commit_hash: "commit_hash".to_string(),
            authorized_principals: vec![self.controller.clone()],
        });

        let storage_canister_id = setup_storage_canister(
            &mut pic,
            self.collection_id,
            storage_init_args,
            self.controller
        );

        pic.tick();
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 30));

        println!("buyback_burn_canister_id: {}", storage_canister_id.to_text());

        TestEnv {
            controller: self.controller,
            nft_owner1: self.nft_owner1,
            nft_owner2: self.nft_owner2,
            storage_canister_id: storage_canister_id,
            pic,
        }
    }
}
