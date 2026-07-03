use core_nft_common::types::icrc37;

pub use icrc37::icrc37_get_collection_approvals::{
    Args0 as GetCollectionApprovalsArgs0, Args1 as GetCollectionApprovalsArgs1,
    Args2 as GetCollectionApprovalsArgs2, Response as GetCollectionApprovalsResponse,
};
pub use icrc37::icrc37_get_token_approvals::{
    Args0 as GetTokenApprovalsArgs0, Args1 as GetTokenApprovalsArgs1,
    Args2 as GetTokenApprovalsArgs2, Response as GetTokenApprovalsResponse,
};
pub use icrc37::icrc37_is_approved::{Args as IsApprovedArgs, Response as IsApprovedResponse};
pub use icrc37::icrc37_max_approvals_per_token_or_collection::{
    Args as MaxApprovalsPerTokenOrCollectionArgs,
    Response as MaxApprovalsPerTokenOrCollectionResponse,
};
pub use icrc37::icrc37_max_revoke_approvals::{
    Args as MaxRevokeApprovalsArgs, Response as MaxRevokeApprovalsResponse,
};
