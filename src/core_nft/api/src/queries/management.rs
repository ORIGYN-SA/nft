pub mod get_public_content_metadata {
    use candid::CandidType;
    use candid::Nat;
    use core_nft_common::types::public_content::EntryDetailResp;
    use serde::{Deserialize, Serialize};

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub token_id: Nat,
        // If provided, return only the named entry; otherwise return all entries.
        pub entry_name: Option<String>,
    }

    pub type Response = Result<Vec<EntryDetailResp>, String>;
}
