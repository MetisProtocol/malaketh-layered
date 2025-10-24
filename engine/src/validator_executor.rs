// Validator Executor module
// Handles pre-execution and post-execution logic for validator set management

use crate::ethereum_rpc::EthereumRPC;
use crate::genesis;
use crate::stake_hub_client::{ElectedValidators, StakeHubClient};
use crate::system_contracts::{SystemContract, STAKE_HUB_CONTRACT};
use color_eyre::eyre::{eyre, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Validator Executor
/// Responsible for:
/// 1. Pre-execution: Reading validator info from StakeHub contract
/// 2. Post-execution: Constructing deposit transaction for coinbase
pub struct ValidatorExecutor {
    /// System contract manager
    system_contract: SystemContract,

    /// StakeHub client for validator set management
    stake_hub_client: StakeHubClient,

    /// Ethereum RPC client
    eth_rpc: Arc<EthereumRPC>,

    /// Current epoch number
    _current_epoch: Arc<RwLock<u64>>,

    /// Epoch length (blocks per epoch)
    epoch_length: u64,

    /// Maximum number of validators
    _max_validators: u64,
}

impl ValidatorExecutor {
    /// Create a new ValidatorExecutor
    pub fn new(eth_rpc: Arc<EthereumRPC>, epoch_length: u64, max_validators: u64) -> Result<Self> {
        let stake_hub_client =
            StakeHubClient::new(eth_rpc.clone(), STAKE_HUB_CONTRACT.parse().unwrap())?;

        Ok(Self {
            system_contract: SystemContract::new()?,
            stake_hub_client,
            eth_rpc,
            _current_epoch: Arc::new(RwLock::new(0)),
            epoch_length,
            _max_validators: max_validators,
        })
    }

    /// Check if current block is at epoch boundary
    pub fn is_epoch_boundary(&self, block_number: u64) -> bool {
        block_number > 0 && block_number % self.epoch_length == 0
    }

    /// Get current epoch from block number
    pub fn get_epoch(&self, block_number: u64) -> u64 {
        block_number / self.epoch_length
    }

    /// Get initial validators from genesis block extraData
    /// This is called at startup to read the validator set from genesis
    pub async fn get_initial_validators_from_genesis(
        &self,
    ) -> Result<crate::system_contracts::ElectedValidators> {
        info!("📖 Reading initial validator set from genesis block extraData...");

        // Step 1: Get genesis block (block 0)
        let genesis_block = self
            .eth_rpc
            .get_block_by_number("0x0")
            .await?
            .ok_or_else(|| eyre!("Genesis block not found"))?;

        info!("✅ Got genesis block, hash: {}", genesis_block.block_hash);

        // Step 2: Parse validator addresses from extraData (extended format with Tendermint keys)
        // Use bytes directly instead of converting to hex string
        let validator_infos =
            genesis::parse_validators_with_tendermint_keys(&genesis_block.extra_data)?;

        info!(
            "✅ Parsed {} validators from extended genesis extraData format",
            validator_infos.len()
        );

        // Extract just the addresses for now
        let validator_addresses: Vec<_> = validator_infos.iter().map(|info| info.address).collect();

        // Step 3: Query StakeHub for each validator's details (tendermint public keys, etc.)
        // For now, we'll return the addresses with default values
        // TODO: Query StakeHub.getValidatorElectionInfo() to get full details

        let validators: Vec<String> = validator_addresses
            .iter()
            .map(|addr| format!("{:?}", addr))
            .collect();

        // Default voting power (equal for all validators initially)
        let voting_powers = vec![1u64; validator_addresses.len()];

        // Empty vote addresses and tendermint pub keys for now
        // These should be queried from StakeHub after init() is called
        // let vote_addrs = vec![String; validator_addresses.len()];
        let tendermint_pub_keys = vec![vec![]; validator_addresses.len()];

        info!("✅ Initial validator set prepared:");
        for (i, validator) in validators.iter().enumerate() {
            info!(
                "  Validator #{}: Address={}, VotingPower={}",
                i + 1,
                validator,
                voting_powers[i]
            );
        }

        Ok(crate::system_contracts::ElectedValidators {
            consensus_addrs: validators,
            voting_powers,
            operator_addrs: vec![], // TODO
            tendermint_pub_keys,
        })
    }

    /// Get current validators from StakeHub contract
    /// This reads the validator set directly from StakeHub without sending update transactions
    pub async fn get_current_validators_from_stake_hub(&self) -> Result<ElectedValidators> {
        info!("Reading current validator set from StakeHub contract...");

        // Use StakeHub client to get top validators by voting power
        let elected_validators = self
            .stake_hub_client
            .get_top_validators_by_voting_power()
            .await?;

        info!(
            "✅ Retrieved {} validators from StakeHub",
            elected_validators.consensus_addrs.len()
        );

        // Convert to the format expected by the system
        Ok(elected_validators)
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
