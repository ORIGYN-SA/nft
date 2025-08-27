use crate::index_suite::setup::setup_core::setup_core_canister;
use crate::index_suite::setup::setup_index::setup_index_canister;
use crate::utils::random_principal;
use bity_ic_types::{CanisterId, Milliseconds};
use candid::{CandidType, Deserialize, Principal};
use core_nft::init::InitArgs;
use core_nft::lifecycle::Args;
use index_icrc7::lifecycle::Args as IndexArgs;
use index_icrc7::lifecycle::InitArgs as IndexInitArgs;
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
    pub index_canister_id: CanisterId,
    pub pic: PocketIc,
}

use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
impl Debug for TestEnv {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestEnv")
            .field("index_id", &self.index_canister_id.to_text())
            .field("collection_id", &self.collection_canister_id.to_text())
            .finish()
    }
}
pub struct TestEnvBuilder {
    pub controller: Principal,
    nft_owner1: Principal,
    nft_owner2: Principal,
    collection_id: CanisterId,
    index_id: CanisterId,
}

impl Default for TestEnvBuilder {
    fn default() -> Self {
        Self {
            controller: random_principal(),
            nft_owner1: random_principal(),
            nft_owner2: random_principal(),
            collection_id: Principal::from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            index_id: Principal::from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
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

    pub fn build(
        &mut self,
        mut init_args_index: IndexInitArgs,
        init_args_collection: InitArgs,
    ) -> TestEnv {
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
        self.index_id = pic.create_canister_with_settings(Some(self.controller.clone()), None);

        pic.tick();
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 10));

        println!("collection_id: {}", self.collection_id.to_text());

        init_args_index
            .authorized_principals
            .push(self.controller.clone());
        init_args_index.ledger_canister_id = self.collection_id;

        let nft_init_args = Args::Init(init_args_collection);
        let index_init_args = IndexArgs::Init(init_args_index);

        println!("nft_init_args: {:?}", nft_init_args);
        let collection_canister_id =
            setup_core_canister(&mut pic, self.collection_id, nft_init_args, self.controller);
        println!(
            "collection_canister_id: {}",
            collection_canister_id.to_text()
        );
        println!("index_init_args: {:?}", index_init_args);
        let index_canister_id =
            setup_index_canister(&mut pic, self.index_id, index_init_args, self.controller);
        println!("index_canister_id: {}", index_canister_id.to_text());

        pic.tick();
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 30));

        TestEnv {
            controller: self.controller,
            nft_owner1: self.nft_owner1,
            nft_owner2: self.nft_owner2,
            collection_canister_id: collection_canister_id,
            index_canister_id: index_canister_id,
            pic,
        }
    }
}
