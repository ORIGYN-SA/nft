use crate::state::read_state;
use candid::Nat;
use core_nft_common::types::icrc7;
use core_nft_common::utils::trace;

pub fn check_memo(memo: &Option<serde_bytes::ByteBuf>) -> Result<(), String> {
    if let Some(ref memo) = memo {
        let max_memo_size: usize = usize::try_from(
            read_state(|state| {
                state
                    .data
                    .max_memo_size
                    .clone()
                    .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_MEMO_SIZE))
            })
            .0,
        )
        .unwrap();

        trace(&format!("Memo Size: {}", memo.len()));
        trace(&format!("Max Memo Size: {}", max_memo_size));

        if memo.len() > max_memo_size {
            trace("Exceeds Max Memo Size");
            return Err("Exceeds Max Memo Size".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {}
