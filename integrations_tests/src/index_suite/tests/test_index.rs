use crate::client::indexer::get_blocks;
use crate::index_suite::setup::setup::MINUTE_IN_MS;
use crate::utils::{mint_nft, tick_n_blocks};
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use icrc_ledger_types::icrc1::account::Account;
use std::time::Duration;

use crate::index_suite::setup::default_test_setup;
use crate::index_suite::setup::setup::TestEnv;
use index_icrc7::types::get_blocks::Args;

#[test]
fn test_icrc7_transfer() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
        index_canister_id,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
        vec![("name".to_string(), Icrc3Value::Text("test".to_string()))],
    );

    tick_n_blocks(pic, 10);
    pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 10));

    let blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 10,
            filter: None,
            sort_by: None,
        },
    );
    println!("blocks: {:?}", blocks.blocks);

    tick_n_blocks(pic, 10);
    pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 10));
}
