use candid::Principal;
use canfund::{
    manager::{
        options::{CyclesThreshold, FundManagerOptions, FundStrategy},
        RegisterOpts,
    },
    operations::fetch::FetchCyclesBalanceFromCanisterStatus,
    FundManager,
};
use std::sync::Arc;

use crate::utils::trace;

pub fn add_canisters_to_fund_manager(
    fund_manager: &mut FundManager,
    canister_id_lst: Vec<Principal>,
) {
    trace(&format!(
        "Adding canisters to fund manager : {:?}",
        canister_id_lst
    ));
    fund_manager.stop();

    let funding_config = FundManagerOptions::new()
        .with_interval_secs(60)
        .with_strategy(FundStrategy::BelowThreshold(
            CyclesThreshold::new()
                .with_min_cycles(1_000_000_000_000)
                .with_fund_cycles(2_000_000_000_000),
        ));

    fund_manager.with_options(funding_config);

    for canister_id in canister_id_lst {
        fund_manager.register(
            canister_id,
            RegisterOpts::new()
                .with_cycles_fetcher(Arc::new(FetchCyclesBalanceFromCanisterStatus::new())),
        );
    }

    fund_manager.start();
}
