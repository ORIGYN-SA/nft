pub use crate::types::icrc37::{
    icrc37_get_collection_approvals, icrc37_get_token_approvals, icrc37_is_approved,
    icrc37_max_approvals_per_token_or_collection, icrc37_max_revoke_approvals,
};
use bity_ic_icrc3::utils::trace;
use candid::Nat;
use ic_cdk_macros::query;
pub use icrc_ledger_types::icrc1::account::Account;

use crate::state::read_state;

#[query]
fn icrc37_get_token_approvals(
    args: icrc37_get_token_approvals::Args,
) -> icrc37_get_token_approvals::Response {
    read_state(|state| {
        let mut response = Vec::new();
        let (token_id, prev, take) = args;

        let take_usize = take
            .map(|n| usize::try_from(n.0).unwrap_or(10))
            .unwrap_or(10);

        if let Some(approvals_map) = state.data.token_approvals.get(&token_id) {
            let mut all_approvals: Vec<icrc37_get_token_approvals::TokenApproval> = approvals_map
                .iter()
                .map(
                    |(account, approval)| icrc37_get_token_approvals::TokenApproval {
                        token_id: token_id.clone(),
                        approval_info: crate::types::icrc37::ApprovalInfo {
                            spender: approval.spender.clone(),
                            from_subaccount: account.subaccount,
                            expires_at: approval.expires_at,
                            memo: approval
                                .memo
                                .as_ref()
                                .map(|m| serde_bytes::ByteBuf::from(m.clone())),
                            created_at_time: approval.created_at,
                        },
                    },
                )
                .collect();

            all_approvals.sort_by(|a, b| {
                b.approval_info
                    .created_at_time
                    .cmp(&a.approval_info.created_at_time)
            });

            let start_index = if let Some(prev_approval) = prev {
                all_approvals
                    .iter()
                    .position(|approval| {
                        approval.approval_info.spender == prev_approval.approval_info.spender
                    })
                    .map(|pos| pos + 1)
                    .unwrap_or(0)
            } else {
                0
            };

            let paginated_approvals = all_approvals
                .iter()
                .skip(start_index)
                .take(take_usize)
                .cloned()
                .collect::<Vec<_>>();

            response.extend(paginated_approvals);
        }

        response
    })
}

#[query]
fn icrc37_get_collection_approvals(
    args: icrc37_get_collection_approvals::Args,
) -> icrc37_get_collection_approvals::Response {
    read_state(|state| {
        let mut response = Vec::new();
        let take_usize = args
            .2
            .map(|n| usize::try_from(n.0).unwrap_or(10))
            .unwrap_or(10);

        trace(&format!(
            "state
            .data
            .collection_approvals: {:?}",
            state.data.collection_approvals
        ));

        let mut all_approvals: Vec<icrc37_get_collection_approvals::CollectionApproval> = state
            .data
            .collection_approvals
            .iter()
            .filter(|(from_account, _)| {
                trace(&format!(
                    "from_account: {:?}, args.0: {:?}",
                    from_account, args.0
                ));
                from_account.owner == args.0.owner && args.0.subaccount == from_account.subaccount
            })
            .map(
                |(from_account, approval)| icrc37_get_collection_approvals::CollectionApproval {
                    approval_info: crate::types::icrc37::ApprovalInfo {
                        spender: approval.spender.clone(),
                        from_subaccount: from_account.subaccount,
                        expires_at: approval.expires_at,
                        memo: approval
                            .memo
                            .as_ref()
                            .map(|m| serde_bytes::ByteBuf::from(m.clone())),
                        created_at_time: approval.created_at,
                    },
                },
            )
            .collect();

        all_approvals.sort_by(|a, b| {
            b.approval_info
                .created_at_time
                .cmp(&a.approval_info.created_at_time)
        });

        trace(&format!("all_approvals: {:?}", all_approvals));

        let start_index = if let Some(prev_approval) = args.1 {
            all_approvals
                .iter()
                .position(|approval| {
                    approval.approval_info.spender == prev_approval.approval_info.spender
                })
                .map(|pos| pos + 1)
                .unwrap_or(0)
        } else {
            0
        };

        let paginated_approvals = all_approvals
            .iter()
            .skip(start_index)
            .take(take_usize)
            .cloned()
            .collect::<Vec<_>>();

        response.extend(paginated_approvals);

        response
    })
}

#[query]
fn icrc37_is_approved(args: icrc37_is_approved::Args) -> icrc37_is_approved::Response {
    read_state(|state| {
        let mut response = Vec::with_capacity(args.len());

        for arg in args {
            let spender_account = arg.spender.clone();
            let from_account = Account {
                owner: arg.spender.owner,
                subaccount: arg.from_subaccount,
            };

            let current_time = ic_cdk::api::time();

            let has_token_approval =
                if let Some(token_approvals) = state.data.token_approvals.get(&arg.token_id) {
                    trace(&format!("token_approvals: {:?}", token_approvals));
                    if let Some(approval) = token_approvals.get(&spender_account) {
                        trace(&format!("approval: {:?}", approval));
                        if let Some(expires_at) = approval.expires_at {
                            trace(&format!("expires_at: {:?}", expires_at));
                            expires_at > current_time
                        } else {
                            trace(&format!("no expires_at"));
                            true
                        }
                    } else {
                        trace(&format!("no approval"));
                        false
                    }
                } else {
                    trace(&format!("no token_approvals"));
                    false
                };

            trace(&format!("has_token_approval: {:?}", has_token_approval));

            let has_collection_approval =
                if let Some(approval) = state.data.collection_approvals.get(&spender_account) {
                    if let Some(expires_at) = approval.expires_at {
                        expires_at > current_time
                    } else {
                        true
                    }
                } else {
                    false
                };

            trace(&format!(
                "has_collection_approval: {:?}",
                has_collection_approval
            ));

            let owner = state.data.owner_of(&arg.token_id);
            let is_owner = owner == Some(from_account);

            trace(&format!("is_owner: {:?}", is_owner));

            response.push(is_owner || has_token_approval || has_collection_approval);
        }

        response
    })
}

#[query]
fn icrc37_max_approvals_per_token_or_collection(
) -> icrc37_max_approvals_per_token_or_collection::Response {
    read_state(|state| {
        state
            .data
            .approval_init
            .max_approvals_per_token_or_collection
            .clone()
    })
}

#[query]
fn icrc37_max_revoke_approvals() -> icrc37_max_revoke_approvals::Response {
    read_state(|state| state.data.approval_init.max_revoke_approvals.clone())
}
