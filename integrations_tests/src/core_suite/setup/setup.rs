use crate::core_suite::setup::setup_core::setup_core_canister;
use crate::utils::random_principal;
use bity_ic_types::{CanisterId, Milliseconds};
use candid::{CandidType, Deserialize, Principal};
use core_nft::init::InitArgs;
use core_nft::lifecycle::Args;
use pocket_ic::{PocketIc, PocketIcBuilder};

use std::time::Duration;

pub const SECOND_IN_MS: Milliseconds = 1000;
pub const MINUTE_IN_MS: Milliseconds = SECOND_IN_MS * 60;
pub const HOUR_IN_MS: Milliseconds = MINUTE_IN_MS * 60;
pub const DAY_IN_MS: Milliseconds = HOUR_IN_MS * 24;

#[derive(CandidType, Deserialize, Debug)]
pub struct RegisterDappCanisterRequest {
    pub canister_id: Option<Principal>,
}

pub struct TestEnv {
    pub controller: Principal,
    pub nft_owner1: Principal,
    pub nft_owner2: Principal,
    pub collection_canister_id: CanisterId,
    pub pic: PocketIc,
}

use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
impl Debug for TestEnv {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestEnv")
            .field("buyback_burn_id", &self.collection_canister_id.to_text())
            .finish()
    }
}
pub struct TestEnvBuilder {
    pub controller: Principal,
    nft_owner1: Principal,
    nft_owner2: Principal,
    collection_id: CanisterId,
    registry_id: CanisterId,
}

impl Default for TestEnvBuilder {
    fn default() -> Self {
        Self {
            controller: random_principal(),
            nft_owner1: random_principal(),
            nft_owner2: random_principal(),
            collection_id: Principal::from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            registry_id: Principal::from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
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

    pub fn build(&mut self, init_args: InitArgs) -> TestEnv {
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
        self.registry_id = pic
            .create_canister_with_id(
                Some(self.controller.clone()),
                None,
                Principal::from_text("rwlgt-iiaaa-aaaaa-aaaaa-cai").unwrap(),
            )
            .unwrap();

        pic.tick();
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 10));

        println!("collection_id: {}", self.collection_id.to_text());

        let nft_init_args = Args::Init(init_args);

        let collection_canister_id = setup_core_canister(
            &mut pic,
            self.collection_id,
            self.registry_id,
            nft_init_args,
            self.controller,
        );

        pic.tick();
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 30));

        println!(
            "buyback_burn_canister_id: {}",
            collection_canister_id.to_text()
        );

        TestEnv {
            controller: self.controller,
            nft_owner1: self.nft_owner1,
            nft_owner2: self.nft_owner2,
            collection_canister_id: collection_canister_id,
            pic,
        }
    }
}
