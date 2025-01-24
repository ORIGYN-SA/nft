use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

mod build_version;
mod http;
mod neuron_info;
mod proposals;
mod rewards_recipients;
mod token;

pub use build_version::*;
pub use http::*;
pub use neuron_info::*;
pub use proposals::*;
pub use rewards_recipients::*;
pub use token::*;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Default)]
pub struct Empty {}

pub type CanisterId = Principal;
pub type CanisterWasm = Vec<u8>;
pub type Cycles = u64;
pub type Hash = [u8; 32];
pub type Maturity = u64;
pub type Milliseconds = u64;
pub type NnsNeuronId = u64;
pub type ProposalId = u64;
pub type SnsNeuronId = [u8; 32];
pub type TimestampSeconds = u64;
pub type TimestampMillis = u64;
pub type TimestampNanos = u64;
