use ic_cdk_macros::update;
use crate::types::{ management, nft, icrc7 };
use candid::Nat;
use crate::guards::caller_is_governance_principal;
use crate::state::{ read_state, mutate_state };
use crate::utils::{ check_memo, hash_string_to_u64 };
use ic_cdk::api::call::RejectionCode;

//TODO Use minting autority to mint tokens
#[update(guard = "caller_is_governance_principal")]
pub fn mint(req: management::mint::Args) -> management::mint::Response {
    let token_name_hash = Nat::from(hash_string_to_u64(&req.token_name));

    let token_list = read_state(|state| { state.data.tokens_list.clone() });
    let supply_cap = read_state(|state| {
        state.data.supply_cap.clone().unwrap_or(Nat::from(icrc7::DEFAULT_MAX_SUPPLY_CAP))
    });

    if token_list.len() > supply_cap {
        return Err((RejectionCode::CanisterError, "Exceed Max allowed Supply Cap".to_string()));
    }

    match check_memo(req.memo) {
        Ok(_) => {}
        Err(e) => {
            return Err((RejectionCode::CanisterError, e));
        }
    }

    match token_list.contains_key(&token_name_hash.clone()) {
        true => {
            return Err((RejectionCode::CanisterError, "Token already exists".to_string()));
        }
        false => {
            let new_token = nft::Icrc7Token::new(
                token_name_hash.clone(),
                req.token_name,
                req.token_description,
                req.token_logo,
                req.token_owner
            );
            mutate_state(|state| {
                state.data.tokens_list.insert(token_name_hash.clone(), new_token);
            });

            // TODO add transactions logs
        }
    }

    Ok(token_name_hash.clone())
}

#[update(guard = "caller_is_governance_principal")]
pub async fn update_nft_metadata(
    req: management::update_nft_metadata::Args
) -> management::update_nft_metadata::Response {
    let token_name_hash = req.token_id;

    let token_list = read_state(|state| { state.data.tokens_list.clone() });

    match token_list.contains_key(&token_name_hash.clone()) {
        true => {
            let mut token = token_list.get(&token_name_hash.clone()).unwrap().clone();
            if let Some(name) = req.token_name {
                token.token_name = name;
            }
            if let Some(description) = req.token_description {
                token.token_description = Some(description);
            }
            if let Some(logo) = req.token_logo {
                token.token_logo = Some(logo);
            }
            if let Some(metadata) = req.token_metadata {
                token.add_metadata(metadata).await;
            }
            mutate_state(|state| {
                state.data.tokens_list.insert(token_name_hash.clone(), token);
            });
        }
        false => {
            return Err((RejectionCode::CanisterError, "Token does not exist".to_string()));
        }
    }

    // TODO add transactions logs

    Ok(token_name_hash.clone())
}

// #[update(guard = "caller_is_governance_principal")]
// pub fn burn_nft() -> () {
//     token.burn()
// }

#[update(guard = "caller_is_governance_principal")]
pub fn update_minting_authorities(
    req: management::update_minting_authorities::Args
) -> management::update_minting_authorities::Response {
    let mut minting_authorities = req.minting_authorities.clone();
    let previous_minting_authorities = mutate_state(|state| {
        state.data.minting_authorities.clone()
    });

    minting_authorities.append(&mut previous_minting_authorities.clone());
    minting_authorities.sort();
    minting_authorities.dedup();

    mutate_state(|state| {
        state.data.minting_authorities = minting_authorities.into_iter().collect();
    });

    Ok(())
}

#[update(guard = "caller_is_governance_principal")]
pub fn remove_minting_authorities(
    req: management::remove_minting_authorities::Args
) -> management::remove_minting_authorities::Response {
    let mut minting_authorities = req.minting_authorities.clone();
    let previous_minting_authorities = mutate_state(|state| {
        state.data.minting_authorities.clone()
    });

    minting_authorities.retain(|auth| !previous_minting_authorities.contains(auth));

    mutate_state(|state| {
        state.data.minting_authorities = minting_authorities.into_iter().collect();
    });

    Ok(())
}
