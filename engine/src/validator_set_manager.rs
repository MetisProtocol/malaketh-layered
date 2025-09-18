use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{engine_rpc::EngineRPC, ethereum_rpc::EthereumRPC};
use alloy_sol_types::{sol, SolCall};
use base64::Engine;
use color_eyre::eyre::{eyre, Result};
use malachitebft_core_types::VotingPower;
use malachitebft_eth_types::{Address, PublicKey};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

sol! {
    contract ValidatorSetManager {
        // Event definitions
        event ValidatorAdded(address indexed validator, uint256 votingPower);
        event ValidatorRemoved(address indexed validator);
        event ValidatorUpdated(
            address indexed validator,
            uint256 oldPower,
            uint256 newPower
        );
        event EpochUpdated(uint256 indexed epoch, address[] validators);
        event Slash(address indexed validator, uint256 amount, string reason);
        event FeeDistributed(address indexed validator, uint256 amount);
        event ProxyUpgraded(
            address indexed oldImplementation,
            address indexed newImplementation
        );

        // Struct definitions
        struct ValidatorInfo {
            address validator;
            uint256 votingPower;
            uint256 stakedAmount;
            bool isActive;
            uint256 lastUpdateEpoch;
            uint256 totalRewards;
            uint256 slashCount;
            bytes32 publicKey;
        }

        // State variables
        mapping(address => ValidatorInfo) public validators;
        mapping(uint256 => address[]) public epochValidators;
        address[] public activeValidators;
        uint256 public currentEpoch;
        uint256 public epochLength;
        uint256 public minStakeAmount;
        uint256 public totalStaked;
        address public admin;
        address public implementation;
        address public proxyAdmin;

        // Modifiers
        modifier onlyAdmin();
        modifier onlyProxyAdmin();

        // Initialization functions
        function initialize(
            address[] calldata initialValidators,
            uint256[] calldata initialPowers,
            bytes32[] calldata initialPublicKeys,
            uint256 _epochLength,
            uint256 _minStakeAmount
        ) external;

        // Staking functions
        function stake(bytes32 publicKey) external payable;
        function unstake(uint256 amount) external;

        // Validator Set management
        function updateValidatorSet() external;

        // Slashing mechanism
        function slashValidator(
            address validator,
            uint256 amount,
            string calldata reason
        ) external;

        // Fee distribution
        function distributeFees() external payable;

        // Query functions
        function getCurrentValidatorSet()
            external
            view
            returns (address[] memory, uint256[] memory);

        function getCurrentValidatorSetWithKeys()
            external
            view
            returns (address[] memory, uint256[] memory, bytes32[] memory);

        function getValidatorInfo(
            address validator
        ) external view returns (ValidatorInfo memory);

        function getEpochLength() external view returns (uint256);
        function getActiveValidatorCount() external view returns (uint256);
        function getTotalStaked() external view returns (uint256);

        // Management functions
        function setEpochLength(uint256 newLength) external;
        function setMinStakeAmount(uint256 newAmount) external;

        // Proxy pattern implementation
        function upgradeTo(address newImplementation) external;
        function setProxyAdmin(address newAdmin) external;

        // Internal functions
        function _addValidator(
            address validator,
            uint256 votingPower,
            uint256 stakedAmount
        ) internal;

        function _removeValidator(address validator) internal;
        function _updateEpochValidators() internal;
    }
}

/// Validator information
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub address: Address,
    pub voting_power: VotingPower,
    pub staked_amount: u64,
    pub is_active: bool,
    pub last_update_epoch: u64,
    pub total_rewards: u64,
    pub slash_count: u64,
    pub public_key: [u8; 32],
}

/// Dynamic validator set manager
pub struct DynamicValidatorSetManager {
    eth_rpc: EthereumRPC,
    contract_address: Address,
    current_epoch: u64,
    epoch_length: u64,
    last_update_height: u64,
    validator_cache: HashMap<Address, ValidatorInfo>,
    update_interval: Duration,
    genesis_validator_set: Option<malachitebft_eth_types::ValidatorSet>,
}

impl DynamicValidatorSetManager {
    pub fn new(eth_rpc: EthereumRPC, contract_address: Address, update_interval: Duration) -> Self {
        Self {
            eth_rpc,
            contract_address,
            current_epoch: 0,
            epoch_length: 100, // Default 100 blocks per epoch
            last_update_height: 0,
            validator_cache: HashMap::new(),
            update_interval,
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

        // Get current validator set from contract
        match self.fetch_validator_set_from_contract().await {
            Ok(validators) => {
                info!("Fetched {} validators from contract", validators.len());
                for validator in validators {
                    self.validator_cache.insert(validator.address, validator);
                }
            }
            Err(e) => {
                warn!(
                    "Failed to fetch validator set from contract: {}, using genesis validators",
                    e
                );
                // Fallback to genesis validators
                match self.get_genesis_validators() {
                    Ok(genesis_validators) => {
                        for validator in genesis_validators {
                            self.validator_cache.insert(validator.address, validator);
                        }
                    }
                    Err(e) => {
                        error!("Failed to load genesis validators: {}", e);
                        // If even genesis validators fail to load, use empty validator set
                    }
                }
            }
        }

        info!("Initialized with {} validators", self.validator_cache.len());
        Ok(())
    }

    /// Check if validator set needs to be updated
    pub async fn should_update_validator_set(&self, current_height: u64) -> bool {
        // Check if epoch boundary is reached
        if current_height % self.epoch_length == 0 && current_height > self.last_update_height {
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

        // Check if contract epoch update needs to be triggered
        if current_height % self.epoch_length == 0 {
            info!("Epoch boundary reached, triggering contract update");
            self.trigger_contract_epoch_update().await?;
        }

        // Get latest validator set from contract
        let validators = self.fetch_validator_set_from_contract().await?;

        // Update cache
        self.validator_cache.clear();
        for validator in &validators {
            self.validator_cache
                .insert(validator.address, validator.clone());
        }

        self.last_update_height = current_height;
        self.current_epoch = current_height / self.epoch_length;

        info!(
            "Updated validator set: {} validators, epoch {}",
            validators.len(),
            self.current_epoch
        );

        Ok(validators)
    }

    /// Get genesis validators
    /// Get validator information from the provided genesis validator set
    fn get_genesis_validators(&self) -> Result<Vec<ValidatorInfo>> {
        if let Some(ref genesis_validator_set) = self.genesis_validator_set {
            let validators =
                self.convert_genesis_validators_to_validator_info(genesis_validator_set);
            if !validators.is_empty() {
                info!(
                    "Loaded {} genesis validators from provided genesis validator set",
                    validators.len()
                );
                return Ok(validators);
            }
        }

        // If no genesis validator set provided, return error
        Err(eyre!("No genesis validator set provided"))
    }

    /// Convert genesis validator set to ValidatorInfo
    fn convert_genesis_validators_to_validator_info(
        &self,
        genesis_validator_set: &malachitebft_eth_types::ValidatorSet,
    ) -> Vec<ValidatorInfo> {
        genesis_validator_set
            .validators
            .iter()
            .map(|validator| {
                ValidatorInfo {
                    address: validator.address,
                    voting_power: validator.voting_power,
                    staked_amount: 1000000000000000000, // 1 ETH - default stake amount
                    is_active: true,
                    last_update_epoch: 0,
                    total_rewards: 0,
                    slash_count: 0,
                    public_key: *validator.public_key.as_bytes(),
                }
            })
            .collect()
    }

    /// Trigger epoch update in contract
    async fn trigger_contract_epoch_update(&self) -> Result<()> {
        info!("Triggering contract epoch update");

        // Construct updateValidatorSet call
        let update_call = ValidatorSetManager::updateValidatorSetCall {};
        let call_data = update_call.abi_encode();

        // Send transaction through Engine API
        match self.send_contract_transaction(call_data).await {
            Ok(tx_hash) => {
                info!("Contract epoch update transaction sent: {:?}", tx_hash);

                // Wait for transaction confirmation
                if let Err(e) = self.wait_for_transaction_confirmation(tx_hash).await {
                    warn!("Failed to wait for transaction confirmation: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to send contract epoch update transaction: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Get cached validator information
    pub fn get_cached_validator(&self, address: &Address) -> Option<&ValidatorInfo> {
        self.validator_cache.get(address)
    }

    /// Get all cached validators
    pub fn get_cached_validators(&self) -> Vec<ValidatorInfo> {
        self.validator_cache.values().cloned().collect()
    }

    /// Get current epoch
    pub fn get_current_epoch(&self) -> u64 {
        self.current_epoch
    }

    /// Get epoch length
    pub fn get_epoch_length_value(&self) -> u64 {
        self.epoch_length
    }

    /// Check if validator is active
    pub fn is_validator_active(&self, address: &Address) -> bool {
        self.validator_cache
            .get(address)
            .map(|v| v.is_active)
            .unwrap_or(false)
    }

    /// Get validator voting power
    pub fn get_validator_voting_power(&self, address: &Address) -> VotingPower {
        self.validator_cache
            .get(address)
            .map(|v| v.voting_power)
            .unwrap_or(0)
    }

    /// Get total voting power
    pub fn get_total_voting_power(&self) -> VotingPower {
        self.validator_cache.values().map(|v| v.voting_power).sum()
    }

    /// Clean up expired validator cache
    pub fn cleanup_expired_validators(&mut self, current_epoch: u64) {
        let expired_epochs = 10; // Keep data for the last 10 epochs
        let cutoff_epoch = current_epoch.saturating_sub(expired_epochs);

        self.validator_cache
            .retain(|_, validator| validator.last_update_epoch >= cutoff_epoch);
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
            let validator = ValidatorInfo {
                address: Address::new(decoded._0[i].into()),
                voting_power: decoded._1[i].to::<u64>(),
                staked_amount: 0, // Needs separate query
                is_active: true,  // Needs separate query
                last_update_epoch: self.current_epoch,
                total_rewards: 0, // Needs separate query
                slash_count: 0,   // Needs separate query
                public_key: decoded._2[i].into(),
            };

            // Output detailed information for each validator
            let public_key_base64 =
                base64::engine::general_purpose::STANDARD.encode(validator.public_key);
            info!(
                "Validator {}: address={}, voting_power={}, public_key={}",
                i + 1,
                validator.address,
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

    /// Send contract transaction
    async fn send_contract_transaction(&self, call_data: Vec<u8>) -> Result<[u8; 32]> {
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
    async fn wait_for_transaction_confirmation(&self, tx_hash: [u8; 32]) -> Result<()> {
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
