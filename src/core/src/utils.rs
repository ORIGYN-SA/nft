use candid::Nat;
use candid::Principal;
use icrc_ledger_types::icrc1::account::Account;
use std::time::Duration;

pub const RETRY_DELAY: Duration = Duration::from_secs(5 * 60); // each 5 minutes

pub async fn get_token_balance(ledger_id: Principal) -> Result<Nat, String> {
    icrc_ledger_canister_c2c_client
        ::icrc1_balance_of(
            ledger_id,
            &(Account {
                owner: ic_cdk::api::id(),
                subaccount: None,
            })
        ).await
        .map_err(|e| format!("Failed to fetch token balance: {:?}", e))
}

#[cfg(test)]
mod tests {}
