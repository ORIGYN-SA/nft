use candid::{ CandidType, Principal };
use canister_time::now_nanos;
use serde::{ Deserialize, Serialize };
use types::BuildVersion;
use types::{ CanisterId, Cycles, TimestampMillis, TimestampNanos };

#[derive(Default, CandidType, Serialize, Deserialize, Clone)]
pub struct CanisterEnv {
    test_mode: bool,
    version: BuildVersion,
    commit_hash: String,
}

pub trait Environment {
    fn now_nanos(&self) -> TimestampNanos;
    fn caller(&self) -> Principal;
    fn canister_id(&self) -> CanisterId;
    fn cycles_balance(&self) -> Cycles;

    fn now(&self) -> TimestampMillis {
        self.now_nanos() / 1_000_000
    }

    fn cycles_balance_in_tc(&self) -> f64 {
        (self.cycles_balance() as f64) / 1_000_000_000_000.0
    }
}

impl CanisterEnv {
    pub fn new(test_mode: bool, version: BuildVersion, commit_hash: String) -> Self {
        Self {
            test_mode,
            version,
            commit_hash,
        }
    }

    pub fn is_test_mode(&self) -> bool {
        self.test_mode
    }

    pub fn version(&self) -> BuildVersion {
        self.version
    }

    pub fn set_version(&mut self, version: BuildVersion) {
        self.version = version;
    }

    pub fn commit_hash(&self) -> &str {
        &self.commit_hash
    }

    pub fn set_commit_hash(&mut self, commit_hash: String) {
        self.commit_hash = commit_hash;
    }
}

impl Environment for CanisterEnv {
    fn now_nanos(&self) -> TimestampNanos {
        now_nanos()
    }

    #[cfg(target_arch = "wasm32")]
    fn caller(&self) -> Principal {
        ic_cdk::caller()
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn caller(&self) -> Principal {
        Principal::anonymous()
    }

    #[cfg(target_arch = "wasm32")]
    fn canister_id(&self) -> CanisterId {
        ic_cdk::id()
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn canister_id(&self) -> CanisterId {
        Principal::anonymous()
    }

    #[cfg(target_arch = "wasm32")]
    fn cycles_balance(&self) -> Cycles {
        ic_cdk::api::canister_balance().into()
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn cycles_balance(&self) -> Cycles {
        0
    }
}
