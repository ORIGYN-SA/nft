pub mod get_caller_nft_private_content_access {
    use candid::CandidType;
    use serde::{Deserialize, Serialize};
    use serde_bytes::ByteBuf;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub context: ByteBuf,
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct DeriveVetkeyPublicKeyResp {
        pub public_key: ByteBuf,
    }

    pub type Response = Result<DeriveVetkeyPublicKeyResp, String>;
}
