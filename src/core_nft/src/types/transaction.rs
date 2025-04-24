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
    pub btype: String, // "7mint", "7burn", "7xfer", "7update_token", "37approve", "37approve_coll", "37revoke", "37revoke_coll", "37xfer"
    pub timestamp: u64,
    pub tx: TransactionData,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TransactionData {
    pub tid: Option<Nat>,
    pub from: Option<Account>,
    pub to: Option<Account>,
    pub meta: Option<ICRC3Value>,
    pub memo: Option<ByteBuf>,
    pub created_at_time: Option<Nat>,
    pub spender: Option<Account>,
    pub exp: Option<Nat>, // expiration time for icrc37
}

impl TransactionType for Icrc3Transaction {
    fn validate_transaction_fields(&self) -> Result<(), String> {
        match self.btype.as_str() {
            "7mint" => {
                if self.tx.tid.is_none() {
                    return Err("Token ID is not allowed for mint".to_string());
                }
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
                if self.tx.tid.is_none() {
                    return Err("Token ID is not allowed for burn".to_string());
                }
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
                if self.tx.tid.is_none() {
                    return Err("Token ID is not allowed for transfer".to_string());
                }
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
                if self.tx.tid.is_none() {
                    return Err("Token ID is not allowed for update_token".to_string());
                }
                if self.tx.meta.is_none() {
                    return Err("Meta is required for update_token".to_string());
                }
                if self.tx.to.is_some() {
                    return Err("To is not allowed for update_token".to_string());
                }
            }
            "37approve" => {
                if self.tx.tid.is_none() {
                    return Err("Token ID is not allowed for token approval".to_string());
                }
                if self.tx.from.is_none() {
                    return Err("From is required for token approval".to_string());
                }
                if self.tx.to.is_some() {
                    return Err("To is not allowed for token approval".to_string());
                }
                if self.tx.spender.is_none() {
                    return Err("Spender is required for token approval".to_string());
                }
                if self.tx.meta.is_some() {
                    return Err("Meta is not allowed for token approval".to_string());
                }
            }
            "37approve_coll" => {
                if self.tx.from.is_none() {
                    return Err("From is required for collection approval".to_string());
                }
                if self.tx.to.is_some() {
                    return Err("To is not allowed for collection approval".to_string());
                }
                if let Some(meta) = &self.tx.meta {
                    if let ICRC3Value::Map(map) = meta {
                        if !map.contains_key("spender") {
                            return Err("Spender is required for collection approval".to_string());
                        }
                    } else {
                        return Err("Meta must be a map for collection approval".to_string());
                    }
                } else {
                    return Err("Meta with spender is required for collection approval".to_string());
                }
            }
            "37revoke" => {
                if self.tx.tid.is_none() {
                    return Err("Token ID is required for token revocation".to_string());
                }
                if self.tx.from.is_none() {
                    return Err("From is required for token revocation".to_string());
                }
                if self.tx.to.is_some() {
                    return Err("To is not allowed for token revocation".to_string());
                }
                if self.tx.spender.is_none() {
                    return Err("Spender is required for token revocation".to_string());
                }
                if self.tx.meta.is_some() {
                    return Err("Meta is not allowed for token revocation".to_string());
                }
            }
            "37revoke_coll" => {
                if self.tx.from.is_none() {
                    return Err("From is required for collection revocation".to_string());
                }
                if self.tx.to.is_some() {
                    return Err("To is not allowed for collection revocation".to_string());
                }
                if self.tx.spender.is_none() {
                    return Err("Spender is required for collection revocation".to_string());
                }
                if self.tx.meta.is_some() {
                    return Err("Meta is not allowed for collection revocation".to_string());
                }
            }
            "37xfer" => {
                if self.tx.tid.is_none() {
                    return Err("Token ID is required for transfer from".to_string());
                }
                if self.tx.from.is_none() {
                    return Err("From is required for transfer from".to_string());
                }
                if self.tx.to.is_none() {
                    return Err("To is required for transfer from".to_string());
                }
                if self.tx.spender.is_none() {
                    return Err("Spender is required for transfer from".to_string());
                }
                if self.tx.meta.is_some() {
                    return Err("Meta is not allowed for transfer from".to_string());
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
        if let Some(tid) = &self.tx.tid {
            hasher.update(tid.0.to_bytes_le());
        }
        if let Some(from) = &self.tx.from {
            hasher.update(from.owner.as_slice());
        }
        if let Some(to) = &self.tx.to {
            hasher.update(to.owner.as_slice());
        }
        if let Some(spender) = &self.tx.spender {
            hasher.update(spender.owner.as_slice());
        }
        if let Some(exp) = &self.tx.exp {
            hasher.update(exp.0.to_bytes_le());
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
        if let Some(tid) = tx.tx.tid {
            map.insert("tid".to_string(), ICRC3Value::Nat(tid));
        }
        if let Some(from) = tx.tx.from {
            map.insert("from".to_string(), ICRC3Value::Text(from.owner.to_string()));
        }
        if let Some(spender) = tx.tx.spender {
            map.insert(
                "spender".to_string(),
                ICRC3Value::Text(spender.owner.to_string()),
            );
        }
        if let Some(exp) = tx.tx.exp {
            map.insert("exp".to_string(), ICRC3Value::Nat(exp));
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
