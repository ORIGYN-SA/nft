use candid::Decode;
use ic_cdk::query;

use crate::state::read_state;
pub use crate::types::icrc21;
pub use crate::types::icrc37;
pub use crate::types::icrc7;
pub use crate::types::management;

#[query]
pub fn icrc21_canister_call_consent_message(
    args: icrc21::icrc21_canister_call_consent_message::Args,
) -> icrc21::icrc21_canister_call_consent_message::Response {
    match args.method.as_str() {
        "icrc7_transfer" => handle_transfer_consent(args),
        "icrc37_approve_tokens" => handle_approve_tokens_consent(args),
        "icrc37_approve_collection" => handle_approve_collection_consent(args),
        "icrc37_revoke_token_approvals" => handle_revoke_token_approvals_consent(args),
        "icrc37_revoke_collection_approvals" => handle_revoke_collection_approvals_consent(args),
        "icrc37_transfer_from" => handle_transfer_from_consent(args),
        _ => handle_unsupported_method(args),
    }
}

fn create_consent_info(
    generic_message: String,
    intent: String,
    fields: Vec<(String, String)>,
    metadata: icrc21::icrc21_canister_call_consent_message::icrc21_consent_message_metadata,
) -> icrc21::icrc21_canister_call_consent_message::Response {
    let fields_message =
        icrc21::icrc21_canister_call_consent_message::icrc21_field_display_message {
            intent,
            fields,
        };

    let consent_message = icrc21::icrc21_canister_call_consent_message::icrc21_consent_message {
        generic_display_message: generic_message,
        fields_display_message: fields_message,
    };

    let consent_info = icrc21::icrc21_canister_call_consent_message::icrc21_consent_info {
        consent_message,
        metadata,
    };

    icrc21::icrc21_canister_call_consent_message::Response::Ok(consent_info)
}

fn create_error_response(
    description: String,
) -> icrc21::icrc21_canister_call_consent_message::Response {
    let error_info =
        icrc21::icrc21_canister_call_consent_message::icrc21_error_info { description };
    icrc21::icrc21_canister_call_consent_message::Response::Err(
        icrc21::icrc21_canister_call_consent_message::icrc21_error::UnsupportedCanisterCall(
            error_info,
        ),
    )
}

fn handle_transfer_consent(
    args: icrc21::icrc21_canister_call_consent_message::Args,
) -> icrc21::icrc21_canister_call_consent_message::Response {
    match Decode!(&args.arg, icrc7::icrc7_transfer::Args) {
        Ok(transfer_args) => {
            let mut fields = vec![
                ("Action".to_string(), "NFT Transfer".to_string()),
                ("Method".to_string(), "icrc7_transfer".to_string()),
            ];

            let token_count = transfer_args.len();
            fields.push(("Number of NFTs".to_string(), token_count.to_string()));

            if let Some(arg) = transfer_args.first() {
                fields.push(("Token ID".to_string(), arg.token_id.to_string()));
                fields.push(("To".to_string(), format!("{}", arg.to.owner)));
                if let Some(memo) = &arg.memo {
                    fields.push(("Memo".to_string(), format!("{:?}", memo)));
                }
            }

            let generic_message = if token_count > 1 {
                format!(
                    "You are about to transfer {} NFTs. Method: icrc7_transfer",
                    token_count
                )
            } else {
                "You are about to transfer an NFT. Method: icrc7_transfer".to_string()
            };

            create_consent_info(
                generic_message,
                "NFT Transfer".to_string(),
                fields,
                args.user_preferences.metadata,
            )
        }
        Err(_) => create_error_response("Failed to decode transfer arguments".to_string()),
    }
}

fn handle_approve_tokens_consent(
    args: icrc21::icrc21_canister_call_consent_message::Args,
) -> icrc21::icrc21_canister_call_consent_message::Response {
    match Decode!(&args.arg, icrc37::icrc37_approve_tokens::Args) {
        Ok(approve_args) => {
            let mut fields = vec![
                (
                    "Action".to_string(),
                    "Approve Tokens for Transfer".to_string(),
                ),
                ("Method".to_string(), "icrc37_approve_tokens".to_string()),
            ];

            let token_count = approve_args.len();
            fields.push(("Number of NFTs".to_string(), token_count.to_string()));

            if let Some(arg) = approve_args.first() {
                fields.push(("Token ID".to_string(), arg.token_id.to_string()));
                fields.push((
                    "Spender".to_string(),
                    format!("{}", arg.approval_info.spender.owner),
                ));
                if let Some(expires_at) = arg.approval_info.expires_at {
                    fields.push(("Expires At".to_string(), expires_at.to_string()));
                }
            }

            let generic_message = if token_count > 1 {
                format!(
                    "You are about to approve {} NFTs for transfer. Method: icrc37_approve_tokens",
                    token_count
                )
            } else {
                "You are about to approve an NFT for transfer. Method: icrc37_approve_tokens"
                    .to_string()
            };

            create_consent_info(
                generic_message,
                "Approve Tokens".to_string(),
                fields,
                args.user_preferences.metadata,
            )
        }
        Err(_) => create_error_response("Failed to decode approve_tokens arguments".to_string()),
    }
}

fn handle_approve_collection_consent(
    args: icrc21::icrc21_canister_call_consent_message::Args,
) -> icrc21::icrc21_canister_call_consent_message::Response {
    match Decode!(&args.arg, icrc37::icrc37_approve_collection::Args) {
        Ok(approve_args) => {
            let mut fields = vec![
                (
                    "Action".to_string(),
                    "Approve Entire Collection".to_string(),
                ),
                (
                    "Method".to_string(),
                    "icrc37_approve_collection".to_string(),
                ),
            ];

            if let Some(arg) = approve_args.first() {
                fields.push((
                    "Spender".to_string(),
                    format!("{}", arg.approval_info.spender.owner),
                ));
                if let Some(expires_at) = arg.approval_info.expires_at {
                    fields.push(("Expires At".to_string(), expires_at.to_string()));
                }

                let token_of_owner =
                    read_state(|state| state.data.tokens_balance_of(&arg.approval_info.spender));

                fields.push((
                    "Total NFTs owned in Collection".to_string(),
                    token_of_owner.to_string(),
                ));
            }

            let generic_message = format!("You are about to approve {} to transfer all your nft available in the collection. Method: icrc37_approve_collection", approve_args.first().unwrap().approval_info.spender.owner);

            create_consent_info(
                generic_message,
                "Approve Collection".to_string(),
                fields,
                args.user_preferences.metadata,
            )
        }
        Err(_) => {
            create_error_response("Failed to decode approve_collection arguments".to_string())
        }
    }
}

fn handle_revoke_token_approvals_consent(
    args: icrc21::icrc21_canister_call_consent_message::Args,
) -> icrc21::icrc21_canister_call_consent_message::Response {
    match Decode!(&args.arg, icrc37::icrc37_revoke_token_approvals::Args) {
        Ok(revoke_args) => {
            let mut fields = vec![
                ("Action".to_string(), "Revoke Token Approvals".to_string()),
                (
                    "Method".to_string(),
                    "icrc37_revoke_token_approvals".to_string(),
                ),
            ];

            let token_count = revoke_args.len();
            fields.push(("Number of NFTs".to_string(), token_count.to_string()));

            if let Some(arg) = revoke_args.first() {
                fields.push(("Token ID".to_string(), arg.token_id.to_string()));
                if let Some(spender) = &arg.spender {
                    fields.push(("Spender".to_string(), format!("{}", spender.owner)));
                }
            }

            let generic_message = if token_count > 1 {
                format!("You are about to revoke approvals for {} NFTs. Method: icrc37_revoke_token_approvals", token_count)
            } else {
                "You are about to revoke approval for an NFT. Method: icrc37_revoke_token_approvals"
                    .to_string()
            };

            create_consent_info(
                generic_message,
                "Revoke Token Approvals".to_string(),
                fields,
                args.user_preferences.metadata,
            )
        }
        Err(_) => {
            create_error_response("Failed to decode revoke_token_approvals arguments".to_string())
        }
    }
}

fn handle_revoke_collection_approvals_consent(
    args: icrc21::icrc21_canister_call_consent_message::Args,
) -> icrc21::icrc21_canister_call_consent_message::Response {
    match Decode!(&args.arg, icrc37::icrc37_revoke_collection_approvals::Args) {
        Ok(revoke_args) => {
            let mut fields = vec![
                (
                    "Action".to_string(),
                    "Revoke Collection Approvals".to_string(),
                ),
                (
                    "Method".to_string(),
                    "icrc37_revoke_collection_approvals".to_string(),
                ),
            ];

            if let Some(arg) = revoke_args.first() {
                if let Some(spender) = &arg.spender {
                    fields.push(("Spender".to_string(), format!("{}", spender.owner)));
                }

                let collection_info = read_state(|state| {
                    (
                        state.data.name.clone(),
                        state.data.description.clone(),
                        state.data.total_supply(),
                    )
                });
                fields.push(("Collection Name".to_string(), collection_info.0));
                if let Some(description) = collection_info.1 {
                    fields.push(("Collection Description".to_string(), description));
                }
                fields.push((
                    "Total NFTs in Collection".to_string(),
                    collection_info.2.to_string(),
                ));
            }

            let generic_message = "You are about to revoke collection approvals. Method: icrc37_revoke_collection_approvals".to_string();

            create_consent_info(
                generic_message,
                "Revoke Collection Approvals".to_string(),
                fields,
                args.user_preferences.metadata,
            )
        }
        Err(_) => create_error_response(
            "Failed to decode revoke_collection_approvals arguments".to_string(),
        ),
    }
}

fn handle_transfer_from_consent(
    args: icrc21::icrc21_canister_call_consent_message::Args,
) -> icrc21::icrc21_canister_call_consent_message::Response {
    match Decode!(&args.arg, icrc37::icrc37_transfer_from::Args) {
        Ok(transfer_args) => {
            let mut fields = vec![
                ("Action".to_string(), "Transfer Using Approval".to_string()),
                ("Method".to_string(), "icrc37_transfer_from".to_string()),
            ];

            let token_count = transfer_args.len();
            fields.push(("Number of NFTs".to_string(), token_count.to_string()));

            if let Some(arg) = transfer_args.first() {
                fields.push(("Token ID".to_string(), arg.token_id.to_string()));
                fields.push(("From".to_string(), format!("{}", arg.from.owner)));
                fields.push(("To".to_string(), format!("{}", arg.to.owner)));
                if let Some(memo) = &arg.memo {
                    fields.push(("Memo".to_string(), format!("{:?}", memo)));
                }
            }

            let generic_message = if token_count > 1 {
                format!("You are about to transfer {} NFTs using approval. Method: icrc37_transfer_from", token_count)
            } else {
                "You are about to transfer an NFT using approval. Method: icrc37_transfer_from"
                    .to_string()
            };

            create_consent_info(
                generic_message,
                "Transfer Using Approval".to_string(),
                fields,
                args.user_preferences.metadata,
            )
        }
        Err(_) => create_error_response("Failed to decode transfer_from arguments".to_string()),
    }
}

fn handle_unsupported_method(
    args: icrc21::icrc21_canister_call_consent_message::Args,
) -> icrc21::icrc21_canister_call_consent_message::Response {
    create_error_response(format!("Method '{}' is not supported", args.method))
}
