use anyhow::{anyhow, Result};
use candid::{Encode, Nat, Principal};
use core_nft::types::management::mint;
use ic_agent::Agent;
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc1::account::Account;

pub async fn mint_nft(
    agent: &Agent,
    canister_id: &Principal,
    owner: Principal,
    subaccount: Option<[u8; 32]>,
    metadata: Vec<(String, ICRC3Value)>,
    memo: Option<&str>,
) -> Result<Nat> {
    let mint_args = mint::Args {
        token_owner: Account { owner, subaccount },
        memo: memo.map(|m| serde_bytes::ByteBuf::from(m.as_bytes())),
        metadata,
    };

    let bytes = Encode!(&mint_args)?;
    let response = agent
        .update(canister_id, "mint")
        .with_arg(bytes)
        .call_and_wait()
        .await?;

    let token_id = candid::decode_one::<mint::Response>(&response)?
        .map_err(|e| anyhow!("Mint failed: {:?}", e))?;

    Ok(token_id)
}
