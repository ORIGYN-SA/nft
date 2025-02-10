use candid::Principal;
use pocket_ic::PocketIc;
use rand::{ rng, RngCore };
use types::Cycles;
use icrc_ledger_types::icrc1::account::Account;
use crate::client::core_nft::mint;
use core_nft::types::management::mint::{ Args as MintArgs, Response as MintResponse };
use crate::core_suite::setup::setup::{ MINUTE_IN_MS, TestEnv };
use std::time::Duration;

pub fn random_principal() -> Principal {
    let mut bytes = [0u8; 29];
    rng().fill_bytes(&mut bytes);
    Principal::from_slice(&bytes)
}

pub fn tick_n_blocks(pic: &PocketIc, times: u32) {
    for _ in 0..times {
        pic.tick();
    }
}

pub fn mint_nft(
    pic: &mut PocketIc,
    token_name: String,
    owner: Account,
    controller: Principal,
    collection_canister_id: Principal
) -> MintResponse {
    let mint_args: MintArgs = MintArgs {
        token_name: token_name,
        token_description: Some("description".to_string()),
        token_logo: Some("logo".to_string()),
        token_owner: owner,
        memo: Some(serde_bytes::ByteBuf::from("memo")),
    };

    let mint_call = mint(pic, controller, collection_canister_id, &mint_args);

    pic.tick();
    pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 30));

    return mint_call;
}

pub const T: Cycles = 1_000_000_000_000;
