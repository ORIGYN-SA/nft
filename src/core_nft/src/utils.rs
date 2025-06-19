use crate::state::read_state;
use crate::types::icrc7;
use candid::Nat;

pub fn check_memo(memo: Option<serde_bytes::ByteBuf>) -> Result<(), String> {
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

pub fn trace(msg: &str) {
    unsafe {
        ic0::debug_print(msg.as_ptr() as usize, msg.len() as usize);
    }
}

#[cfg(test)]
mod tests {}
