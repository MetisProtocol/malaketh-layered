//! StakeHub Client module
//! Handles interaction with StakeHub contract for validator election and information retrieval

use crate::ethereum_rpc::EthereumRPC;
use alloy_dyn_abi::{DynSolValue, FunctionExt, JsonAbiExt};
use alloy_json_abi::JsonAbi;
use alloy_primitives::{Address, U256};
use color_eyre::eyre::Result;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;

/// Validator election information from StakeHub contract
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValidatorElectionInfo {
    pub consensus_address: Address,
    pub voting_power: U256,
    pub operator_address: Address,
    pub tendermint_pub_key: Vec<u8>,
}

/// Elected validators result
#[derive(Clone, Debug, Default)]
pub struct ElectedValidators {
    pub consensus_addrs: Vec<Address>,
    pub voting_powers: Vec<u64>,
    pub operator_addrs: Vec<Address>,
    pub tendermint_pub_keys: Vec<Vec<u8>>,
}

impl Ord for ValidatorElectionInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.voting_power.cmp(&other.voting_power) {
            // If the voting power is the same, we compare the address as string.
            Ordering::Equal => other
                .consensus_address
                .to_string()
                .cmp(&self.consensus_address.to_string()),
            other => other,
        }
    }
}

impl PartialOrd for ValidatorElectionInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Client for interacting with StakeHub contract
pub struct StakeHubClient {
    eth_rpc: Arc<EthereumRPC>,
    stake_hub_address: Address,
    stake_hub_abi: JsonAbi,
}

impl StakeHubClient {
    /// Create a new StakeHubClient
    pub fn new(eth_rpc: Arc<EthereumRPC>, stake_hub_address: Address) -> Result<Self> {
        // Load StakeHub ABI from embedded JSON
        let abi_str = include_str!("system_contracts/abis/StakeHub.json");
        let stake_hub_abi: JsonAbi = serde_json::from_str(abi_str)?;

        Ok(Self {
            eth_rpc,
            stake_hub_address,
            stake_hub_abi,
        })
    }

    /// Get epoch length from StakeHub contract
    pub async fn get_epoch_length(&self) -> Result<u64> {
        let function = self
            .stake_hub_abi
            .function("epochLength")
            .unwrap()
            .first()
            .unwrap();

        let call_data = function.abi_encode_input(&[])?;
        let result = self
            .eth_rpc
            .eth_call(&self.stake_hub_address.to_string(), &call_data)
            .await?;

        let output = function.abi_decode_output(&result, false)?;
        let epoch_length: U256 = output[0].as_uint().unwrap().0;
        Ok(epoch_length.to::<u64>())
    }

    /// Get max elected validators from StakeHub contract
    pub async fn get_max_elected_validators(&self) -> Result<U256> {
        let function = self
            .stake_hub_abi
            .function("maxElectedValidators")
            .unwrap()
            .first()
            .unwrap();

        let call_data = function.abi_encode_input(&[])?;
        let result = self
            .eth_rpc
            .eth_call(&self.stake_hub_address.to_string(), &call_data)
            .await?;

        let output = function.abi_decode_output(&result, false)?;
        let max_elected = output[0].as_uint().unwrap().0;

        Ok(max_elected)
    }

    /// Get validator election info from StakeHub contract
    pub async fn get_validator_election_info(
        &self,
    ) -> Result<(Vec<Address>, Vec<U256>, Vec<Address>, Vec<Vec<u8>>, U256)> {
        let function = self
            .stake_hub_abi
            .function("getValidatorElectionInfo")
            .unwrap()
            .first()
            .unwrap();

        let call_data = function.abi_encode_input(&[
            DynSolValue::from(U256::from(0)),
            DynSolValue::from(U256::from(0)),
        ])?;

        let result = self
            .eth_rpc
            .eth_call(&self.stake_hub_address.to_string(), &call_data)
            .await?;
        let output = function.abi_decode_output(&result, false)?;

        let consensus_addresses = output[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|val| val.as_address().unwrap())
            .collect::<Vec<_>>();

        let voting_powers = output[1]
            .as_array()
            .unwrap()
            .iter()
            .map(|val| val.as_uint().unwrap().0)
            .collect();

        let operator_addresses = output[2]
            .as_array()
            .unwrap()
            .iter()
            .map(|val| val.as_address().unwrap())
            .collect();

        let tendermint_pub_keys = output[3]
            .as_array()
            .unwrap()
            .iter()
            .map(|val| val.as_bytes().unwrap().to_vec())
            .collect();

        let total_length = output[4].as_uint().unwrap().0;

        Ok((
            consensus_addresses,
            voting_powers,
            operator_addresses,
            tendermint_pub_keys,
            total_length,
        ))
    }

    /// Get top validators by voting power from StakeHub contract
    pub async fn get_top_validators_by_voting_power(&self) -> Result<ElectedValidators> {
        // Get max elected validators
        let max_elected = self.get_max_elected_validators().await?;

        // Get all validator election info
        let (
            consensus_addresses,
            voting_powers,
            operator_addresses,
            tendermint_pub_keys,
            _total_length,
        ) = self.get_validator_election_info().await?;

        // Convert to ValidatorElectionInfo
        let validators: Vec<ValidatorElectionInfo> = consensus_addresses
            .into_iter()
            .zip(voting_powers.into_iter())
            .zip(operator_addresses.into_iter())
            .zip(tendermint_pub_keys.into_iter())
            .map(
                |(((consensus_address, voting_power), operator_address), tendermint_pub_key)| {
                    ValidatorElectionInfo {
                        consensus_address,
                        voting_power,
                        operator_address,
                        tendermint_pub_key,
                    }
                },
            )
            .collect();

        // Apply the selection algorithm
        let result = get_top_validators_by_voting_power(validators, max_elected);

        Ok(result)
    }
}

/// Get top validators by voting power using binary heap
fn get_top_validators_by_voting_power(
    validators: Vec<ValidatorElectionInfo>,
    max_elected: U256,
) -> ElectedValidators {
    let mut validator_heap: BinaryHeap<ValidatorElectionInfo> = BinaryHeap::new();

    for validator in validators {
        if validator.voting_power > U256::ZERO {
            validator_heap.push(validator);
        }
    }

    let top_n = max_elected.to::<u64>() as usize;
    let top_n = if top_n > validator_heap.len() {
        validator_heap.len()
    } else {
        top_n
    };

    let mut elected_validators = Vec::with_capacity(top_n);
    let mut elected_voting_powers = Vec::with_capacity(top_n);
    let mut elected_operator_addrs = Vec::with_capacity(top_n);
    let mut elected_tendermint_pub_keys = Vec::with_capacity(top_n);

    for _ in 0..top_n {
        if let Some(validator) = validator_heap.pop() {
            elected_validators.push(validator.consensus_address);
            elected_voting_powers
                .push((validator.voting_power / U256::from(10u64.pow(10))).to::<u64>());
            elected_operator_addrs.push(validator.operator_address);
            elected_tendermint_pub_keys.push(validator.tendermint_pub_key);
        }
    }

    ElectedValidators {
        consensus_addrs: elected_validators,
        voting_powers: elected_voting_powers,
        operator_addrs: elected_operator_addrs,
        tendermint_pub_keys: elected_tendermint_pub_keys,
    }
}
