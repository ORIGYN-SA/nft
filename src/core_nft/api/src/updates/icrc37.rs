use crate::types::icrc37;

pub use icrc37::icrc37_approve_collection::{Args as ApproveCollectionArgs, Response as ApproveCollectionResponse};
pub use icrc37::icrc37_approve_tokens::{Args as ApproveTokensArgs, Response as ApproveTokensResponse};
pub use icrc37::icrc37_revoke_collection_approvals::{Args as RevokeCollectionApprovalsArgs, Response as RevokeCollectionApprovalsResponse};
pub use icrc37::icrc37_revoke_token_approvals::{Args as RevokeTokenApprovalsArgs, Response as RevokeTokenApprovalsResponse};
pub use icrc37::icrc37_transfer_from::{Args as TransferFromArgs, Response as TransferFromResponse};