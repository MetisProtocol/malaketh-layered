// Type definitions for system contracts

use serde::{Deserialize, Serialize};

/// Validator information from system contracts
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub consensus_address: String,
    pub operator_address: String,
    pub voting_power: u64,
    pub vote_address: Vec<u8>,
}

/// Validator election information
#[derive(Clone, Debug)]
pub struct ValidatorElectionInfo {
    pub consensus_address: String,
    pub voting_power: u64,
    pub operator_address: String,
    pub tendermint_pub_key: Vec<u8>,
}

/// Elected validators after sorting by voting power
#[derive(Clone, Debug)]
pub struct ElectedValidators {
    pub consensus_addrs: Vec<String>,
    pub voting_powers: Vec<u64>,
    pub operator_addrs: Vec<String>,
    pub tendermint_pub_keys: Vec<Vec<u8>>,
}

/// Get top validators by voting power
pub fn get_top_validators_by_voting_power(
    mut validators: Vec<ValidatorElectionInfo>,
    max_elected: u64,
) -> ElectedValidators {
    // Sort by voting power (descending)
    validators.sort_by(|a, b| {
        match b.voting_power.cmp(&a.voting_power) {
            std::cmp::Ordering::Equal => {
                // If voting power is equal, sort by address (ascending)
                a.consensus_address.cmp(&b.consensus_address)
            }
            other => other,
        }
    });

    // Take top N validators
    let top_n = std::cmp::min(max_elected as usize, validators.len());
    let top_validators: Vec<_> = validators.into_iter().take(top_n).collect();

    ElectedValidators {
        consensus_addrs: top_validators
            .iter()
            .map(|v| v.consensus_address.clone())
            .collect(),
        voting_powers: top_validators
            .iter()
            .map(|v| {
                // Divide by 1e10 for voting power normalization
                v.voting_power / 10_000_000_000
            })
            .collect(),
        operator_addrs: top_validators
            .iter()
            .map(|v| v.operator_address.clone())
            .collect(),
        tendermint_pub_keys: top_validators
            .iter()
            .map(|v| v.tendermint_pub_key.clone())
            .collect(),
    }
}
