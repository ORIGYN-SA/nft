use std::collections::HashSet;

use candid::{CandidType, Principal};
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, CandidType, Debug, Clone, PartialEq, Eq)]
pub struct RewardsRecipient {
    /// The account to which the rewards will be disbursed
    pub account: Account,
    /// A tag to identify the recipient
    pub tag: String,
    /// The weight of the rewards to be disbursed to this recipient. The weight is a number between 1 and 10000.
    /// For consistency, the sum of all weights should add up to 10000. If you are defining % values, define them as
    /// multiples of 100. E.g. 33% would be 3300, 1.5% would be 150 and 75.23% would be 7523.
    pub reward_weight: u16,
}

#[derive(Serialize, Deserialize, Clone, CandidType, PartialEq, Eq, Debug)]
pub struct RewardsRecipientList(Vec<RewardsRecipient>);

impl RewardsRecipientList {
    const TOTAL_REWARD_WEIGHT: u16 = 10000;

    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    pub fn set(&mut self, list: Vec<RewardsRecipient>) -> Result<(), String> {
        Self::validate(&list)?;
        *self = Self(list);
        Ok(())
    }

    pub fn validate(list: &Vec<RewardsRecipient>) -> Result<(), String> {
        if list.is_empty() {
            return Err("Invalid rewards recipients: empty list.".to_string());
        }
        // expecting 4 recipients in the current design. Limit can be lifted if needed
        if list.len() > 5 {
            return Err("Invalid rewards recipients: too many recipients.".to_string());
        }
        // only allow unique accounts
        if !Self::validate_unique_accounts(list) {
            return Err(
                "Only unique accounts may be defined in the recipients list. Found duplicates."
                    .to_string(),
            );
        }
        let mut sum = 0;
        for recipient in list {
            if recipient.account.owner == Principal::anonymous() {
                return Err("Invalid rewards recipient: account owner is anonymous.".to_string());
            }
            if recipient.reward_weight == 0 || recipient.reward_weight > Self::TOTAL_REWARD_WEIGHT {
                return Err(format!(
                    "Invalid rewards recipient: reward weight has to be between 1 and {}.",
                    Self::TOTAL_REWARD_WEIGHT
                )
                .to_string());
            }
            sum += recipient.reward_weight;
        }
        if sum != Self::TOTAL_REWARD_WEIGHT {
            return Err(format!(
                "Invalid rewards recipient: the sum of all needs to add up to {}.",
                Self::TOTAL_REWARD_WEIGHT
            )
            .to_string());
        }
        Ok(())
    }

    pub fn validate_unique_accounts(recipients: &[RewardsRecipient]) -> bool {
        let mut accounts_seen = HashSet::new();
        for recipient in recipients {
            if !accounts_seen.insert(&recipient.account) {
                // duplicate found
                return false;
            }
        }
        // No duplicates found
        true
    }

    pub fn split_amount_to_each_recipient(
        &self,
        amount: u64,
    ) -> Result<Vec<(Account, u64)>, String> {
        if amount < 100_000_000 {
            return Err(format!(
                "Amount needs to be at least 100_000_000 (1 ICP). Passed amount: {}",
                amount
            )
            .to_string());
        }
        if self.0.is_empty() {
            return Err("No reward recipients defined.".to_string());
        }

        let mut result = vec![];
        for recipient in &self.0 {
            // This does a simple calculation of amount * reward_weight / TOTAL_REWARD_WEIGHT, while checking all the boundaries
            let amount_share = amount
                .checked_mul(recipient.reward_weight as u64)
                .ok_or(
                    format!(
                        "Error while multiplying amount with reward_weight for recipient {}.",
                        recipient.tag
                    )
                    .to_string(),
                )
                .and_then(|val| {
                    val.checked_div(Self::TOTAL_REWARD_WEIGHT as u64).ok_or(
                        format!(
                            "Error while diving amount by TOTAL_REWARD_WEIGHT for recipient {}.",
                            recipient.tag
                        )
                        .to_string(),
                    )
                })?;
            result.push((recipient.account, amount_share));
        }
        // As division of unsigned ints always applies the 'floor' rounding, we'll add any remainder to the last
        // recipient in the list. This avoids any dust remaining in dissolved neurons.
        let obtained_sum: u64 = result.iter().map(|(_, val)| val).sum();
        if obtained_sum > amount {
            return Err(
                format!(
                    "Total sum of calculated rewards larger than amount to be split. sum: {obtained_sum}, amount: {amount}"
                ).to_string()
            );
        }
        if obtained_sum < amount {
            let diff = amount - obtained_sum;
            if let Some((_, val)) = result.last_mut() {
                *val += diff;
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use candid::Principal;
    use icrc_ledger_types::icrc1::account::{Account, Subaccount};

    use crate::{RewardsRecipient, RewardsRecipientList};

    #[test]
    fn initialise_rewards_recipient_list_empty() {
        let mut list = RewardsRecipientList::empty();
        let result = list.set(vec![]);

        assert_eq!(
            result,
            Err("Invalid rewards recipients: empty list.".to_string())
        )
    }

    #[test]
    fn initialise_rewards_recipient_list_wrong_sum() {
        let mut list = RewardsRecipientList::empty();
        let result = list.set(vec![
            dummy_recipient(1000, Some([0; 32])),
            dummy_recipient(1000, Some([1; 32])),
        ]);

        assert_eq!(
            result,
            Err(format!(
                "Invalid rewards recipient: the sum of all needs to add up to {}.",
                RewardsRecipientList::TOTAL_REWARD_WEIGHT
            )
            .to_string())
        )
    }

    #[test]
    fn initialise_rewards_recipient_list_valid() {
        let recipients = vec![
            dummy_recipient(3300, Some([0; 32])),
            dummy_recipient(3300, Some([1; 32])),
            dummy_recipient(3300, Some([2; 32])),
            dummy_recipient(100, Some([3; 32])),
        ];

        let mut list = RewardsRecipientList::empty();
        list.set(recipients.clone()).unwrap();

        assert_eq!(list.0, recipients)
    }

    #[test]
    fn split_amount_to_each_recipient_empty_list() {
        let list = RewardsRecipientList::empty();
        let result = list.split_amount_to_each_recipient(100_000_000);

        let expected_result = Err("No reward recipients defined.".to_string());
        assert_eq!(result, expected_result)
    }

    #[test]
    fn split_amount_to_each_recipient() {
        let mut list = RewardsRecipientList::empty();
        list.set(vec![
            dummy_recipient(3300, Some([0; 32])),
            dummy_recipient(3300, Some([1; 32])),
            dummy_recipient(3300, Some([2; 32])),
            dummy_recipient(100, Some([3; 32])),
        ])
        .unwrap();

        let amount1: u64 = 100_000_000_000;

        let result1 = list.split_amount_to_each_recipient(amount1);

        let expected_result1 = Ok(vec![
            (dummy_account(Some([0; 32])), 33_000_000_000 as u64),
            (dummy_account(Some([1; 32])), 33_000_000_000 as u64),
            (dummy_account(Some([2; 32])), 33_000_000_000 as u64),
            (dummy_account(Some([3; 32])), 1_000_000_000 as u64),
        ]);

        assert_eq!(result1, expected_result1);

        let amount2: u64 = 555_555_555;

        let result2 = list.split_amount_to_each_recipient(amount2);

        let expected_result2 = Ok(vec![
            (dummy_account(Some([0; 32])), 183_333_333 as u64),
            (dummy_account(Some([1; 32])), 183_333_333 as u64),
            (dummy_account(Some([2; 32])), 183_333_333 as u64),
            (dummy_account(Some([3; 32])), 5_555_556 as u64),
        ]);

        assert_eq!(result2, expected_result2);
    }
    #[test]
    fn split_amount_to_each_recipient_invalid_amount() {
        let mut list = RewardsRecipientList::empty();
        list.set(vec![
            dummy_recipient(3300, Some([0; 32])),
            dummy_recipient(3300, Some([1; 32])),
            dummy_recipient(3300, Some([2; 32])),
            dummy_recipient(100, Some([3; 32])),
        ])
        .unwrap();

        let amount: u64 = 123456;

        let result = list.split_amount_to_each_recipient(amount);

        assert_eq!(
            result,
            Err(format!(
                "Amount needs to be at least 100_000_000 (1 ICP). Passed amount: {}",
                amount
            )
            .to_string())
        );
    }

    fn dummy_account(subaccount: Option<Subaccount>) -> Account {
        Account {
            owner: Principal::from_text(
                "thrhh-hnmzu-kjquw-6ebmf-vdhed-yf2ry-avwy7-2jrrm-byg34-zoqaz-wqe",
            )
            .unwrap(),
            subaccount,
        }
    }

    fn dummy_recipient(reward_weight: u16, subaccount: Option<Subaccount>) -> RewardsRecipient {
        RewardsRecipient {
            account: dummy_account(subaccount),
            tag: "test".to_string(),
            reward_weight,
        }
    }
}
