use candid::{CandidType, Deserialize};

pub mod icrc10_supported_standards {
    use super::*;

    #[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
    pub struct SupportedStandard {
        pub name: String,
        pub url: String,
    }
    pub type SupportedStandardsResponse = Vec<SupportedStandard>;

    pub type Args = ();
    pub type Response = SupportedStandardsResponse;
}
