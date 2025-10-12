use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::ethereum_rpc::EthereumRPC;
use alloy_sol_types::{sol, SolCall};
use base64::Engine;
use color_eyre::eyre::{eyre, Result};
use malachitebft_core_types::VotingPower;
use malachitebft_eth_types::Address;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

sol! {
    contract ValidatorSetManager {
        struct ValidatorInfo {
            address consensusAddress;  // Tendermint address for consensus
            address operatorAddress;   // Ethereum address for operations
            uint256 votingPower;
            bytes32 publicKey;
        }

        // Modifiers
        modifier onlyAdmin();
        modifier onlyProxyAdmin();

        // Initialization functions
        function initialize(
            address[] calldata consensusAddresses,
            address[] calldata operatorAddresses,
            uint256[] calldata initialPowers,
            bytes32[] calldata initialPublicKeys,
            uint256 _epochLength,
        ) external;

        // Query functions
        function getCurrentValidatorSetWithKeys()
            external
            view
            returns (address[] memory, address[] memory, uint256[] memory, bytes32[] memory);

        function getValidatorInfo(
            address consensusAddress
        ) external view returns (ValidatorInfo memory);

        function getValidatorNum() external view returns (uint256);
        function getEpochLength() external view returns (uint256);
        function getUpdateHeight() external view returns (uint256);
        function getValidatorCount() external view returns (uint256);

        // Management functions
        function setEpochLength(uint256 newLength) external;
        function setValidatorNum(uint256 newValidatorNum) external;

        // Proxy pattern implementation
        function upgradeTo(address newImplementation) external;
        function setProxyAdmin(address newAdmin) external;
        function AddValidatorBase64(
            address validator,
            uint256 votingPower,
            string calldata publicKey
        ) external;
        // Internal functions
        function _base64ToBytes32(string memory base64String) internal pure returns (bytes32);
        function _addValidator(
            address consensusAddress,
            address operatorAddress,
            uint256 votingPower,
            bytes32 publicKey
        ) internal;

        function _removeValidator(address consensusAddress) internal;
    }
}

/// Validator information
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub consensus_address: Address, // Tendermint address for consensus
    pub operator_address: Address,  // Ethereum address for operations
    pub voting_power: VotingPower,
    pub public_key: [u8; 32],
}

/// Dynamic validator set manager
pub struct DynamicValidatorSetManager {
    eth_rpc: EthereumRPC,
    contract_address: Address,
    epoch_length: u64,
    last_update_height: u64,
    genesis_validator_set: Option<malachitebft_eth_types::ValidatorSet>,
}

impl DynamicValidatorSetManager {
    pub fn new(
        eth_rpc: EthereumRPC,
        contract_address: Address,
        _update_interval: Duration,
    ) -> Self {
        Self {
            eth_rpc,
            contract_address,
            epoch_length: 100, // Default 100 blocks per epoch
            last_update_height: 0,
            genesis_validator_set: None,
        }
    }

    pub fn with_genesis_validator_set(
        mut self,
        genesis_validator_set: malachitebft_eth_types::ValidatorSet,
    ) -> Self {
        self.genesis_validator_set = Some(genesis_validator_set);
        self
    }

    /// Initialize the validator set manager
    pub async fn initialize(&mut self) -> Result<()> {
        info!(
            "Initializing DynamicValidatorSetManager for contract: {}",
            self.contract_address
        );

        // Get epoch length from contract
        match self.fetch_epoch_length_from_contract().await {
            Ok(epoch_length) => {
                self.epoch_length = epoch_length;
                info!("Fetched epoch length from contract: {}", self.epoch_length);
            }
            Err(e) => {
                warn!(
                    "Failed to fetch epoch length from contract: {}, using default: {}",
                    e, self.epoch_length
                );
            }
        }

        Ok(())
    }

    /// Check if validator set needs to be updated
    pub async fn should_update_validator_set(&self, current_height: u64) -> bool {
        let validator_count = self
            .fetch_validator_count_from_contract()
            .await
            .unwrap_or(0);
        if validator_count == 0 {
            warn!("Validator contract not available or returned zero validators");
            return false;
        }
        let Ok(validator_num) = self.fetch_validator_num_from_contract().await else {
            warn!("Failed to fetch validator num from contract");
            return false;
        };
        debug!(
            "Judge update validator set! current_height:{}, epoch_len:{}, val_num:{}, val_count:{}, last_updateH:{}",
            current_height, self.epoch_length, validator_num, validator_count, self.last_update_height
        );
        // Check if epoch boundary is reached
        if current_height % self.epoch_length == 0 && current_height > self.last_update_height {
            match self.fetch_update_height_from_contract().await {
                Ok(height) => {
                    // validator_set in state has been updated with contract value
                    if self.last_update_height >= height {
                        debug!("Contract update height {} is less than last update height {}, skipping update", height, self.last_update_height);
                        return false;
                    } else {
                        debug!("Contract update height from contract: {}", height);
                    }
                },
                Err(e) => {
                    warn!("Failed to fetch update height from contract: {}", e);
                    return false;
                }
            };

            return true;
        }

        // Time-based update logic can be added here
        false
    }

    /// Update validator set
    pub async fn update_validator_set(
        &mut self,
        current_height: u64,
    ) -> Result<Vec<ValidatorInfo>> {
        info!("Updating validator set at height {}", current_height);

        // Get latest validator set from contract
        let mut validators = self.fetch_validator_set_from_contract().await?;
        let validator_num = self.fetch_validator_num_from_contract().await? as usize;
        validators.sort_by(|a, b| {
            // sort by voting_power, from high to low
            b.voting_power.cmp(&a.voting_power)
        });
        validators.truncate(validator_num);

        self.last_update_height = current_height;

        info!("Updated validator set: {} validators", validators.len(),);

        Ok(validators)
    }

    /// Get validator number from contract
    pub async fn fetch_validator_num_from_contract(&self) -> Result<u64> {
        let call = ValidatorSetManager::getValidatorNumCall {};
        let call_data = call.abi_encode();

        let result = self
            .eth_rpc
            .call_contract(self.contract_address, call_data)
            .await
            .map_err(|e| eyre!("Failed to call contract: {}", e))?;

        // Check if result is empty
        if result.is_empty() {
            return Err(eyre!("Empty contract response"));
        }

        let decoded = ValidatorSetManager::getValidatorNumCall::abi_decode_returns(&result)
            .map_err(|e| eyre!("Failed to decode contract response: {}", e))?;

        Ok(decoded.to::<u64>())
    }

    /// Get validator count from contract
    pub async fn fetch_validator_count_from_contract(&self) -> Result<u64> {
        let call = ValidatorSetManager::getValidatorCountCall {};
        let call_data = call.abi_encode();

        let result = self
            .eth_rpc
            .call_contract(self.contract_address, call_data)
            .await
            .map_err(|e| eyre!("Failed to call contract: {}", e))?;

        // Check if result is empty
        if result.is_empty() {
            return Err(eyre!("Empty contract response"));
        }

        let decoded = ValidatorSetManager::getValidatorCountCall::abi_decode_returns(&result)
            .map_err(|e| eyre!("Failed to decode contract response: {}", e))?;

        Ok(decoded.to::<u64>())
    }

    /// Get update height from contract
    async fn fetch_update_height_from_contract(&self) -> Result<u64> {
        let call = ValidatorSetManager::getUpdateHeightCall {};
        let call_data = call.abi_encode();

        let result = self
            .eth_rpc
            .call_contract(self.contract_address, call_data)
            .await
            .map_err(|e| eyre!("Failed to call contract: {}", e))?;

        // Check if result is empty
        if result.is_empty() {
            return Err(eyre!("Empty contract response"));
        }

        let decoded = ValidatorSetManager::getUpdateHeightCall::abi_decode_returns(&result)
            .map_err(|e| eyre!("Failed to decode contract response: {}", e))?;

        Ok(decoded.to::<u64>())
    }

    /// Get epoch length from contract
    async fn fetch_epoch_length_from_contract(&self) -> Result<u64> {
        let call = ValidatorSetManager::getEpochLengthCall {};
        let call_data = call.abi_encode();

        let result = self
            .eth_rpc
            .call_contract(self.contract_address, call_data)
            .await
            .map_err(|e| eyre!("Failed to call contract: {}", e))?;

        // Check if result is empty
        if result.is_empty() {
            return Err(eyre!("Empty contract response"));
        }

        let decoded = ValidatorSetManager::getEpochLengthCall::abi_decode_returns(&result)
            .map_err(|e| eyre!("Failed to decode contract response: {}", e))?;

        Ok(decoded.to::<u64>())
    }

    /// Get current validator set from contract
    async fn fetch_validator_set_from_contract(&self) -> Result<Vec<ValidatorInfo>> {
        let call = ValidatorSetManager::getCurrentValidatorSetWithKeysCall {};
        let call_data = call.abi_encode();

        let result = self
            .eth_rpc
            .call_contract(self.contract_address, call_data)
            .await
            .map_err(|e| eyre!("Failed to call contract: {}", e))?;

        // Check if result is empty
        if result.is_empty() {
            return Err(eyre!("Empty contract response"));
        }

        let decoded =
            ValidatorSetManager::getCurrentValidatorSetWithKeysCall::abi_decode_returns(&result)
                .map_err(|e| eyre!("Failed to decode contract response: {}", e))?;

        let mut validators = Vec::new();
        for i in 0..decoded._0.len() {
            let consensus_address = Address::new(decoded._0[i].into());
            let operator_address = Address::new(decoded._1[i].into());

            let validator = ValidatorInfo {
                consensus_address,
                operator_address,
                voting_power: decoded._2[i].to::<u64>(),
                public_key: decoded._3[i].into(),
            };

            // Output detailed information for each validator
            let public_key_base64 =
                base64::engine::general_purpose::STANDARD.encode(validator.public_key);
            info!(
                "Validator {}: consensus_address={}, operator_address={}, voting_power={}, public_key={}",
                i + 1,
                validator.consensus_address,
                validator.operator_address,
                validator.voting_power,
                public_key_base64
            );

            validators.push(validator);
        }

        info!(
            "Successfully fetched {} validators from contract",
            validators.len()
        );
        Ok(validators)
    }

    /// Get epoch length
    pub fn get_epoch_length_value(&self) -> u64 {
        self.epoch_length
    }

    /// Send contract transaction
    async fn _send_contract_transaction(&self, call_data: Vec<u8>) -> Result<[u8; 32]> {
        // Get gas price
        let gas_price = self
            .eth_rpc
            .get_gas_price()
            .await
            .map_err(|e| eyre!("Failed to get gas price: {}", e))?;

        // Construct transaction
        let tx = serde_json::json!({
            "to": format!("0x{}", self.contract_address),
            "data": format!("0x{}", hex::encode(call_data)),
            "gas": "0x5208", // 21000 gas
            "gasPrice": gas_price,
            "value": "0x0"
        });

        // Send transaction
        let response = self
            .eth_rpc
            .send_transaction(tx)
            .await
            .map_err(|e| eyre!("Failed to send transaction: {}", e))?;

        // Parse transaction hash
        let tx_hash_hex = response
            .as_str()
            .ok_or_else(|| eyre!("Invalid transaction hash format"))?;

        let tx_hash = hex::decode(tx_hash_hex.strip_prefix("0x").unwrap_or(tx_hash_hex))
            .map_err(|e| eyre!("Failed to decode transaction hash: {}", e))?;

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&tx_hash[..32]);

        Ok(hash_bytes)
    }

    /// Wait for transaction confirmation
    async fn _wait_for_transaction_confirmation(&self, tx_hash: [u8; 32]) -> Result<()> {
        let tx_hash_hex = format!("0x{}", hex::encode(tx_hash));

        // Poll transaction status, wait up to 30 seconds
        let max_attempts = 30;
        let mut attempts = 0;

        while attempts < max_attempts {
            match self.eth_rpc.get_transaction_receipt(&tx_hash_hex).await {
                Ok(Some(receipt)) => {
                    if receipt.status == Some("0x1".to_string()) {
                        info!("Transaction confirmed: {}", tx_hash_hex);
                        return Ok(());
                    } else {
                        return Err(eyre!("Transaction failed: {}", tx_hash_hex));
                    }
                }
                Ok(None) => {
                    // Transaction still pending, continue waiting
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    attempts += 1;
                }
                Err(e) => {
                    warn!("Failed to get transaction receipt: {}, retrying...", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    attempts += 1;
                }
            }
        }

        Err(eyre!("Transaction confirmation timeout: {}", tx_hash_hex))
    }
}

/// Validator set update event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorSetUpdateEvent {
    pub epoch: u64,
    pub height: u64,
    pub validators: Vec<ValidatorInfo>,
    pub timestamp: u64,
}

impl ValidatorSetUpdateEvent {
    pub fn new(epoch: u64, height: u64, validators: Vec<ValidatorInfo>) -> Self {
        Self {
            epoch,
            height,
            validators,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}
