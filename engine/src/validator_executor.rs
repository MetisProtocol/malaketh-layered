// Validator Executor module
// Handles pre-execution and post-execution logic for validator set management

use crate::ethereum_rpc::EthereumRPC;
use crate::stake_hub_client::StakeHubClient;
use crate::system_contracts::STAKE_HUB_CONTRACT;
use color_eyre::eyre::Result;
use std::sync::Arc;
use tracing::{info, warn};

/// Validator Executor
pub struct ValidatorExecutor {
    /// StakeHub client for validator set management
    stake_hub_client: StakeHubClient,
}

impl ValidatorExecutor {
    /// Create a new ValidatorExecutor
    pub fn new(eth_rpc: Arc<EthereumRPC>) -> Result<Self> {
        let stake_hub_client =
            StakeHubClient::new(eth_rpc.clone(), STAKE_HUB_CONTRACT.parse().unwrap())?;

        Ok(Self { stake_hub_client })
    }

    /// Check if current block is at epoch boundary
    pub async fn is_epoch_boundary(&self, block_number: u64, epoch_length: u64) -> bool {
        block_number > 0 && block_number % epoch_length == 0
    }

    /// Get epoch length from StakeHub contract
    pub async fn get_epoch_length_from_stake_hub(&self) -> Result<u64> {
        self.stake_hub_client.get_epoch_length().await
    }

    /// Get validator set from StakeHub contract and convert to ValidatorSet format
    /// This is a higher-level function that returns a ValidatorSet for consensus
    pub async fn get_validator_set_from_stake_hub(
        &self,
    ) -> Result<Option<malachitebft_eth_types::ValidatorSet>> {
        // Get top validators by voting power
        match self
            .stake_hub_client
            .get_top_validators_by_voting_power()
            .await
        {
            Ok(elected_validators) => {
                info!(
                    "✅ Retrieved {} validators from StakeHub",
                    elected_validators.consensus_addrs.len()
                );

                // Convert to ValidatorSet format
                let validators: Vec<malachitebft_eth_types::Validator> = elected_validators
                    .consensus_addrs
                    .into_iter()
                    .zip(elected_validators.voting_powers.into_iter())
                    .zip(elected_validators.operator_addrs.into_iter())
                    .zip(elected_validators.tendermint_pub_keys.into_iter())
                    .map(
                        |(((consensus_addr, voting_power), operator_addr), tendermint_pub_key)| {
                            let consensus_addr =
                                malachitebft_eth_types::Address::from(consensus_addr);
                            let operator_addr =
                                malachitebft_eth_types::Address::from(operator_addr);
                            let public_key = malachitebft_eth_types::PublicKey::from_bytes(
                                tendermint_pub_key.try_into().unwrap(),
                            );

                            malachitebft_eth_types::Validator {
                                consensus_address: consensus_addr,
                                operator_address: operator_addr,
                                public_key,
                                voting_power: voting_power as u64,
                            }
                        },
                    )
                    .collect();

                Ok(Some(malachitebft_eth_types::ValidatorSet::new(validators)))
            }
            Err(e) => {
                warn!("Failed to get validators from StakeHub: {}", e);
                Ok(None)
            }
        }
    }
}
