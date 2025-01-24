use candid::{CandidType, Nat};
use icrc_ledger_types::icrc2::allowance::{Allowance, AllowanceArgs};
use serde::Deserialize;

pub type Args = AllowanceArgs;

pub type Response = Allowance;
