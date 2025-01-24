use ic_cdk_macros::update;
use crate::types::icrc7;
use candid::Nat;

#[update]
pub fn icrc7_transfer(arg: icrc7::TransferArg) -> icrc7::TransferResult {
    // check token exist
    // check token owner
    Ok(Nat::from(0 as u64))
}
