use candid::CandidType;
use serde::{ Deserialize, Serialize };

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CollectionMetadata {}
