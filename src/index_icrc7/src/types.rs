pub mod get_blocks {
    use crate::index::{IndexType, SortBy};
    use candid::CandidType;
    use icrc_ledger_types::icrc3::blocks::BlockWithId;
    use serde::{Deserialize, Serialize};

    #[derive(CandidType, Deserialize, Serialize)]
    pub struct Args {
        pub start: u64,
        pub length: u64,
        pub filter: Option<IndexType>,
        pub sort_by: Option<SortBy>,
    }

    #[derive(CandidType, Deserialize, Serialize)]
    pub struct Response {
        pub blocks: Vec<BlockWithId>,
    }
}
