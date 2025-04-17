use bity_ic_icrc3::transaction::{Hash, TransactionType};
use bity_ic_types::TimestampSeconds;

use candid::{CandidType, Nat};
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Icrc3Transaction {
    pub btype: String, // "7mint", "7burn", "7xfer", "7update_token"
    pub timestamp: u64,
    pub tx: TransactionData,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TransactionData {
    pub tid: Nat,
    pub from: Option<Account>,
    pub to: Option<Account>,
    pub meta: Option<ICRC3Value>,
    pub memo: Option<ByteBuf>,
    pub created_at_time: Option<Nat>,
}

impl TransactionType for Icrc3Transaction {
    fn validate_transaction_fields(&self) -> Result<(), String> {
        match self.btype.as_str() {
            "7mint" => {
                if self.tx.from.is_some() {
                    return Err("From is not allowed for mint".to_string());
                }
                if self.tx.to.is_none() {
                    return Err("To is required for mint".to_string());
                }
                if self.tx.meta.is_some() {
                    return Err("Meta is not allowed for mint".to_string());
                }
            }
            "7burn" => {
                if self.tx.from.is_none() {
                    return Err("From is required for burn".to_string());
                }
                if self.tx.to.is_some() {
                    return Err("To is not allowed for burn".to_string());
                }
                if self.tx.meta.is_some() {
                    return Err("Meta is not allowed for burn".to_string());
                }
            }
            "7xfer" => {
                if self.tx.from.is_none() {
                    return Err("From is required for transfer".to_string());
                }
                if self.tx.to.is_none() {
                    return Err("To is required for transfer".to_string());
                }
                if self.tx.meta.is_some() {
                    return Err("Meta is not allowed for transfer".to_string());
                }
            }
            "7update_token" => {
                if self.tx.meta.is_none() {
                    return Err("Meta is required for update_token".to_string());
                }
                if self.tx.to.is_some() {
                    return Err("To is not allowed for update_token".to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn timestamp(&self) -> Option<TimestampSeconds> {
        Some(self.timestamp)
    }

    fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.btype.as_bytes());
        hasher.update(self.timestamp.to_le_bytes().as_slice());
        hasher.update(self.tx.tid.0.to_bytes_le());
        if let Some(from) = &self.tx.from {
            hasher.update(from.owner.as_slice());
        }
        if let Some(to) = &self.tx.to {
            hasher.update(to.owner.as_slice());
        }
        if let Some(meta) = &self.tx.meta {
            hasher.update(serde_cbor::to_vec(meta).unwrap_or_default());
        }
        if let Some(memo) = &self.tx.memo {
            hasher.update(memo.as_slice());
        }
        if let Some(time) = &self.tx.created_at_time {
            hasher.update(time.0.to_bytes_le());
        }
        hasher.finalize().into()
    }

    fn block_type(&self) -> String {
        self.btype.clone()
    }
}

impl From<Icrc3Transaction> for ICRC3Value {
    fn from(tx: Icrc3Transaction) -> Self {
        let mut map = BTreeMap::new();
        map.insert("btype".to_string(), ICRC3Value::Text(tx.btype));
        map.insert(
            "timestamp".to_string(),
            ICRC3Value::Nat(Nat::from(tx.timestamp)),
        );
        map.insert("tid".to_string(), ICRC3Value::Nat(tx.tx.tid));
        if let Some(from) = tx.tx.from {
            map.insert("from".to_string(), ICRC3Value::Text(from.owner.to_string()));
        }
        if let Some(to) = tx.tx.to {
            map.insert("to".to_string(), ICRC3Value::Text(to.owner.to_string()));
        }
        if let Some(meta) = tx.tx.meta {
            map.insert("meta".to_string(), meta);
        }
        if let Some(memo) = tx.tx.memo {
            map.insert("memo".to_string(), ICRC3Value::Blob(memo));
        }
        if let Some(time) = tx.tx.created_at_time {
            map.insert("created_at_time".to_string(), ICRC3Value::Nat(time));
        }
        ICRC3Value::Map(map)
    }
}
