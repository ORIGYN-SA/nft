use std::{borrow::Cow, fmt::Display};

use candid::{CandidType, Decode, Encode, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Serialize, Clone, Deserialize, CandidType, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct TokenSymbol(String);

#[derive(Debug)]
pub enum TokenSymbolParseError {
    InvalidTokenSymbol,
}

impl Display for TokenSymbolParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTokenSymbol => write!(f, "InvalidTokenSymbol"),
        }
    }
}

const MAX_VALUE_SIZE: u32 = 12;
impl TokenSymbol {
    pub fn parse(symbol: &str) -> Result<TokenSymbol, TokenSymbolParseError> {
        const ALLOWED_TOKENS: [&str; 3] = ["ICP", "OGY", "GLDGov"];

        let valid_token = ALLOWED_TOKENS.contains(&symbol);
        if valid_token {
            Ok(TokenSymbol(symbol.to_string()))
        } else {
            Err(TokenSymbolParseError::InvalidTokenSymbol)
        }
    }
}

impl Storable for TokenSymbol {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(&bytes, Self).unwrap()
    }
    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_VALUE_SIZE,
        is_fixed_size: false,
    };
}

#[derive(Debug, Serialize, Clone, Deserialize, CandidType, PartialEq, Eq, Hash, Copy)]
pub struct TokenInfo {
    pub ledger_id: Principal,
    pub fee: u64,
    pub decimals: u64,
}
