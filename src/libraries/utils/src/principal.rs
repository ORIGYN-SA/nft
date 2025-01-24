use candid::Principal;
use icrc_ledger_types::icrc1::account::Account;
use std::str::FromStr;

pub trait PrincipalDotAccountFormat {
    fn to_principal_dot_account(&self) -> String;
}

impl PrincipalDotAccountFormat for Account {
    fn to_principal_dot_account(&self) -> String {
        match &self.subaccount {
            Some(subaccount) => format!("{}.{}", self.owner, hex::encode(subaccount)),
            None => format!(
                "{}.0000000000000000000000000000000000000000000000000000000000000000",
                self.owner.to_string()
            ),
        }
    }
}
pub fn string_to_account(input: String) -> Result<Account, String> {
    if let Some(index) = input.find('.') {
        let (principal_str, subaccount_str) = input.split_at(index);
        let subaccount_str: String = subaccount_str.chars().skip(1).collect();
        match Principal::from_str(principal_str) {
            Ok(valid_principal) => {
                let valid_subaccount_str = if subaccount_str.len() < 64 {
                    format!("{:0>64}", subaccount_str)
                } else {
                    subaccount_str
                };
                match hex::decode(valid_subaccount_str) {
                    Ok(decoded_subaccount) => {
                        if decoded_subaccount.len() == 32 {
                            let mut subaccount_array = [0u8; 32];
                            subaccount_array.copy_from_slice(&decoded_subaccount);

                            let principal_with_subaccount = Account {
                                owner: valid_principal,
                                subaccount: Some(subaccount_array),
                            };
                            Ok(principal_with_subaccount)
                        } else {
                            Err(
                                "split_into_principal_and_account -> subaccount length check, expected 32 bytes".to_string()
                            )
                        }
                    }
                    Err(err) => {
                        let err_message = format!(
                            "split_into_principal_and_account -> hex::decode(subaccount_value){err:?}"
                        );
                        Err(err_message)
                    }
                }
            }
            Err(err) => Err(err.to_string()),
        }
    } else {
        match Principal::from_str(input.as_str()) {
            Ok(valid_principal) => Ok(Account {
                owner: valid_principal,
                subaccount: None,
            }),
            Err(err) => Err(err.to_string()),
        }
    }
}

pub fn validate_principal_dot_account(input: &str) -> Option<String> {
    match string_to_account(input.to_string()) {
        Ok(account) => Some(account.to_principal_dot_account()),
        Err(_) => match Account::from_str(input) {
            Ok(account) => Some(account.to_principal_dot_account()),
            Err(err) => {
                println!("{err:?}");
                None
            }
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_principal_dot_account_with_checksum() {
        let input =
            "yuijc-oiaaa-aaaap-ahezq-cai-7qfaeci.100000000000000000000000000000000000000000000000000000000000000";
        let expected =
            "yuijc-oiaaa-aaaap-ahezq-cai.0100000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(
            validate_principal_dot_account(input),
            Some(expected.to_string())
        );
    }

    #[test]
    fn test_validate_principal_dot_account_without_checksum() {
        let input =
            "yuijc-oiaaa-aaaap-ahezq-cai.100000000000000000000000000000000000000000000000000000000000000";
        let expected =
            "yuijc-oiaaa-aaaap-ahezq-cai.0100000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(
            validate_principal_dot_account(input),
            Some(expected.to_string())
        );
    }

    #[test]
    fn test_validate_principal_dot_account_invalid_format() {
        let input = "invalid-format";
        assert_eq!(validate_principal_dot_account(input), None);
    }

    #[test]
    fn test_validate_principal_dot_account_short_account_id() {
        let input = "yuijc-oiaaa-aaaap-ahezq-cai.1";
        let expected =
            "yuijc-oiaaa-aaaap-ahezq-cai.0000000000000000000000000000000000000000000000000000000000000001";
        assert_eq!(
            validate_principal_dot_account(input),
            Some(expected.to_string())
        );
    }
}
