use anyhow::{anyhow, Result};
use candid::{Encode, Nat, Principal};
use core_nft_common::types::management::mint;
use core_nft_common::PrivateEntry;
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
    private_content: Option<PrivateEntry>,
) -> Result<Nat> {
    let mint_args = mint::Args {
        mint_requests: vec![mint::MintRequest {
            token_owner: Account { owner, subaccount },
            memo: memo.map(|m| serde_bytes::ByteBuf::from(m.as_bytes())),
            metadata,
            private_content,
        }],
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
