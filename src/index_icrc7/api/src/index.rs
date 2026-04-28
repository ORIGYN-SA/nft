use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};
use icrc_ledger_types::icrc1::account::Account;

#[derive(
    CandidType, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq, Clone, Debug,
)]
pub enum SortBy {
    Ascending,
    Descending,
}

#[derive(
    CandidType, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq, Clone, Debug,
)]
pub enum IndexType {
    Account(WrappedAccount),
    BlockType(String),
    TokenId(WrappedNat),
}

#[derive(CandidType, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq, Clone, Debug)]
pub struct WrappedAccount(pub Account);

#[derive(CandidType, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq, Clone, Debug)]
pub struct WrappedNat(pub Nat);

impl From<Account> for WrappedAccount {
    fn from(account: Account) -> Self {
        WrappedAccount(account)
    }
}

impl From<Nat> for WrappedNat {
    fn from(nat: Nat) -> Self {
        WrappedNat(nat)
    }
}