use std::slice;
use std::sync::Arc;

use malachitebft_core_types::VotingPower;
use serde::{Deserialize, Serialize};

use crate::signing::PublicKey;
use crate::{Address, TestContext};

pub struct ValidatorSol {
    pub public_key: String,
    pub voting_power: u64,
}

impl ValidatorSol {
    pub fn to_validator(&self) -> Validator {
        let pub_key_json = format!(
            r#"{{"type": "tendermint/PubKeyEd25519", "value": "{}"}}"#,
            self.public_key
        );
        Validator::new(
            serde_json::from_str(&pub_key_json).expect("Invalid public key"),
            self.voting_power,
        )
    }
}

/// A validator is a public key and voting power
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Validator {
    pub address: Address,
    pub public_key: PublicKey,
    pub voting_power: VotingPower,
}

impl Validator {
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn new(public_key: PublicKey, voting_power: VotingPower) -> Self {
        Self {
            address: Address::from_public_key(&public_key),
            public_key,
            voting_power,
        }
    }
}

impl PartialOrd for Validator {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Validator {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.address.cmp(&other.address)
    }
}

impl malachitebft_core_types::Validator<TestContext> for Validator {
    fn address(&self) -> &Address {
        &self.address
    }

    fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    fn voting_power(&self) -> VotingPower {
        self.voting_power
    }
}

/// A validator set contains a list of validators sorted by address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorSet {
    pub validators: Arc<Vec<Validator>>,
}

impl ValidatorSet {
    pub fn new(validators: impl IntoIterator<Item = Validator>) -> Self {
        let mut validators: Vec<_> = validators.into_iter().collect();
        ValidatorSet::sort_validators(&mut validators);

        assert!(!validators.is_empty());

        Self {
            validators: Arc::new(validators),
        }
    }

    /// Get the number of validators in the set
    pub fn len(&self) -> usize {
        self.validators.len()
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }

    /// Iterate over the validators in the set
    pub fn iter(&self) -> slice::Iter<'_, Validator> {
        self.validators.iter()
    }


    /// The total voting power of the validator set
    pub fn total_voting_power(&self) -> VotingPower {
        self.validators.iter().map(|v| v.voting_power).sum()
    }

    /// Get a validator by its address
    pub fn get_by_address(&self, address: &Address) -> Option<&Validator> {
        self.validators.iter().find(|v| &v.address == address)
    }

    pub fn get_by_public_key(&self, public_key: &PublicKey) -> Option<&Validator> {
        self.validators.iter().find(|v| &v.public_key == public_key)
    }

    /// In place sort and deduplication of a list of validators
    fn sort_validators(vals: &mut Vec<Validator>) {
        // Sort the validators according to the current Tendermint requirements
        //
        // use core::cmp::Reverse;
        //
        // (v. 0.34 -> first by validator power, descending, then by address, ascending)
        // vals.sort_unstable_by(|v1, v2| {
        //     let a = (Reverse(v1.voting_power), &v1.address);
        //     let b = (Reverse(v2.voting_power), &v2.address);
        //     a.cmp(&b)
        // });

        vals.dedup();
    }
    pub fn get_keys(&self) -> Vec<PublicKey> {
        self.validators.iter().map(|v| v.public_key).collect()
    }
}

impl malachitebft_core_types::ValidatorSet<TestContext> for ValidatorSet {
    fn count(&self) -> usize {
        self.validators.len()
    }

    fn total_voting_power(&self) -> VotingPower {
        self.total_voting_power()
    }

    fn get_by_address(&self, address: &Address) -> Option<&Validator> {
        self.get_by_address(address)
    }

    fn get_by_index(&self, index: usize) -> Option<&Validator> {
        self.validators.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALIDATOR_SET_JSON: &str = r#"
    {
        "validators": [
            {
                "address": "0x0754445aeda0441230d3ab099b0942181915186c",
                "public_key": {
                    "type": "tendermint/PubKeyEd25519",
                    "value": "lwB6erO0yiT4uI5tzrdk/ov/gQv0X8Fu978JQfy9eic="
                },
                "voting_power": 1
            },
            {
                "address": "0x3f8f2908b1b5b6ef3eec1968fcdf8340a6bec221",
                "public_key": {
                    "type": "tendermint/PubKeyEd25519",
                    "value": "2sSy+F3l4EwwGgd7CCVvZZ3d82o5V4NhsZmd9WI3q44="
                },
                "voting_power": 1
            },
            {
                "address": "0x9ab1a8b89460fccd8eb6739352300988915c71fe",
                "public_key": {
                    "type": "tendermint/PubKeyEd25519",
                    "value": "G0lKW8Y0v6FAwfW492XHwCA6XTpziDVC7D3Q2q/DYVc="
                },
                "voting_power": 1
            }
        ]
    }
    "#;

    #[test]
    fn test_validator_set() {
        let validator_set: ValidatorSet = serde_json::from_str(VALIDATOR_SET_JSON).unwrap();
        
        assert_eq!(validator_set.validators.len(), 3);
        assert_eq!(validator_set.validators[0].voting_power, 1);
    }

    #[test]
    fn test_validator_sol() {
        let validator_sol = ValidatorSol {
            public_key: "lwB6erO0yiT4uI5tzrdk/ov/gQv0X8Fu978JQfy9eic=".to_string(),
            voting_power: 1,
        };
        let validator = validator_sol.to_validator();
        assert_eq!(validator.voting_power, 1);
        assert_eq!(
            validator.public_key,
            serde_json::from_str(
                r#"{
                    "type": "tendermint/PubKeyEd25519",
                    "value": "lwB6erO0yiT4uI5tzrdk/ov/gQv0X8Fu978JQfy9eic="
                }"#
            )
            .unwrap()
        );

        let validator_set: ValidatorSet = serde_json::from_str(VALIDATOR_SET_JSON).unwrap();
        assert_eq!(validator_set.validators[0].public_key, validator.public_key);
    }
}
