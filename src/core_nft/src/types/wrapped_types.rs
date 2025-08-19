use crate::types::icrc37::{Approval, TokenApprovalValue};

use candid::{CandidType, Nat, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use icrc_ledger_types::icrc1::account::Account;
use minicbor::Encoder;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap};

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct WrappedNat(pub Nat);

impl From<Nat> for WrappedNat {
    fn from(nat: Nat) -> Self {
        WrappedNat(nat)
    }
}

impl Storable for WrappedNat {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buffer = Vec::new();
        minicbor::encode(self, &mut buffer).expect("failed to encode Nat");
        Cow::Owned(buffer)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        minicbor::decode(&bytes).expect("failed to decode WrappedNat")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl<C> minicbor::Encode<C> for WrappedNat {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.u64(u64::try_from(self.0 .0.clone()).unwrap())?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for WrappedNat {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let nat = d.u64()?;
        Ok(WrappedNat(Nat::from(nat)))
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Ord, PartialOrd)]
pub struct WrappedAccount(pub Account);

impl From<Account> for WrappedAccount {
    fn from(account: Account) -> Self {
        WrappedAccount(account)
    }
}

impl Storable for WrappedAccount {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buffer = Vec::new();
        minicbor::encode(self, &mut buffer).expect("failed to encode Account");
        Cow::Owned(buffer)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        minicbor::decode(&bytes).expect("failed to decode WrappedAccount")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl Eq for WrappedAccount {}

impl<C> minicbor::Encode<C> for WrappedAccount {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(2)?;
        e.bytes(self.0.owner.as_slice())?;
        match &self.0.subaccount {
            Some(subaccount) => e.bytes(subaccount.as_slice())?,
            None => e.null()?,
        };
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for WrappedAccount {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let array = d.array()?.unwrap();
        if array != 2 {
            return Err(minicbor::decode::Error::message(
                "expected array of length 2",
            ));
        }
        let owner = d.bytes()?;
        let subaccount = if d.datatype()? == minicbor::data::Type::Null {
            d.null()?;
            None
        } else {
            Some(d.bytes()?)
        };
        Ok(WrappedAccount(Account {
            owner: Principal::from_slice(&owner),
            subaccount: subaccount.map(|subaccount| {
                let mut subaccount_array = [0u8; 32];
                subaccount_array.copy_from_slice(&subaccount);
                subaccount_array
            }),
        }))
    }
}

pub struct WrappedApprovalValue(pub TokenApprovalValue);

impl Storable for WrappedApprovalValue {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buffer = Vec::new();
        minicbor::encode(self, &mut buffer).expect("failed to encode TokenApprovalValue");
        Cow::Owned(buffer)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        minicbor::decode(&bytes).expect("failed to decode TokenApprovalValue")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl<C> minicbor::Encode<C> for WrappedApprovalValue {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.map(self.0.len() as u64)?;
        for (k, v) in self.0.iter() {
            k.encode(e, _ctx)?;
            v.encode(e, _ctx)?;
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for WrappedApprovalValue {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let map = d.map()?.unwrap();
        let mut new_map = HashMap::new();

        for _ in 0..map {
            let key = WrappedAccount::decode(d, ctx)?;
            let value = Approval::decode(d, ctx)?;
            new_map.insert(key, value);
        }
        Ok(WrappedApprovalValue(new_map))
    }
}
