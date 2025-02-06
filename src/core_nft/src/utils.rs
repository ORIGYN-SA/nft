use std::hash::{ Hash, Hasher };
use std::collections::hash_map::DefaultHasher;

use crate::state::read_state;
use crate::types::icrc7;
use candid::Nat;

pub fn hash_string_to_u64(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

pub fn check_memo(memo: Option<serde_bytes::ByteBuf>) -> Result<(), String> {
    if let Some(ref memo) = memo {
        let max_memo_size: usize = usize
            ::try_from(
                read_state(|state|
                    state.data.max_memo_size
                        .clone()
                        .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_MEMO_SIZE))
                ).0
            )
            .unwrap();

        if memo.len() > max_memo_size {
            return Err("Exceeds Max Memo Size".to_string());
        }
    }
    Ok(())
}

pub fn trace(msg: &str) {
    unsafe {
        ic0::debug_print(msg.as_ptr() as i32, msg.len() as i32);
    }
}

#[cfg(test)]
mod tests {}
