use candid::Principal;
use ic_ledger_types::{AccountIdentifier, Subaccount, DEFAULT_SUBACCOUNT};
use icrc_ledger_types::icrc1::account::Account;
use sha2::{Digest, Sha256};

pub fn compute_neuron_staking_subaccount_bytes(controller: Principal, nonce: u64) -> [u8; 32] {
    const DOMAIN: &[u8] = b"neuron-stake";
    const DOMAIN_LENGTH: [u8; 1] = [0x0c];

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_LENGTH);
    hasher.update(DOMAIN);
    hasher.update(controller.as_slice());
    hasher.update(nonce.to_be_bytes());
    hasher.finalize().into()
}

pub fn icrc_account_to_legacy_account_id(icrc_account: Account) -> AccountIdentifier {
    let subaccount: Subaccount = icrc_account
        .subaccount
        .map_or(DEFAULT_SUBACCOUNT, |s| Subaccount(s));
    AccountIdentifier::new(&icrc_account.owner, &subaccount)
}

pub fn principal_to_legacy_account_id(
    principal: Principal,
    subaccount: Option<Subaccount>,
) -> AccountIdentifier {
    AccountIdentifier::new(&principal, &subaccount.unwrap_or(DEFAULT_SUBACCOUNT))
}

#[cfg(test)]
mod tests {
    use candid::Principal;
    use icrc_ledger_types::icrc1::account::Account;

    use crate::icrc_account_to_legacy_account_id;

    #[test]
    fn convert_icrc_account_to_legacy_account_id() {
        let icrc_account = Account {
            owner: Principal::from_text(
                "465sx-szz6o-idcax-nrjhv-hprrp-qqx5e-7mqwr-wadib-uo7ap-lofbe-dae",
            )
            .unwrap(),
            subaccount: None,
        };
        let result = icrc_account_to_legacy_account_id(icrc_account);

        let expected_result =
            "aacba041bbce2b03c66307a68ca2d5a704a1f87397694a1292d89ce757136f11".to_string();

        assert_eq!(result.to_hex(), expected_result)
    }

    #[test]
    fn convert_icrc_account_to_legacy_account_id_with_subaccount() {
        let icrc_account = Account {
            owner: Principal::from_text(
                "465sx-szz6o-idcax-nrjhv-hprrp-qqx5e-7mqwr-wadib-uo7ap-lofbe-dae",
            )
            .unwrap(),
            subaccount: Some([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
        };
        let result = icrc_account_to_legacy_account_id(icrc_account);

        let expected_result =
            "2fab56f6af866bd4580c8bdf821849d470d5d0af6a671191c602e0c434b5e55c".to_string();

        assert_eq!(result.to_hex(), expected_result)
    }
}
