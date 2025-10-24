// System Contracts integration module
// Provides SystemContract structure to interact with on-chain system contracts

use color_eyre::eyre::Result;
use serde_json::Value;

pub mod abi;
pub mod types;

pub use types::*;

// Contract addresses (configurable genesis contracts)
pub const VALIDATOR_CONTRACT: &str = "0x0000000000000000000000000000000000001000";
pub const SLASH_CONTRACT: &str = "0x0000000000000000000000000000000000001001";
pub const SYSTEM_REWARD_CONTRACT: &str = "0x0000000000000000000000000000000000001002";
pub const STAKE_HUB_CONTRACT: &str = "0x0000000000000000000000000000000000002002";

// System address for internal calls
pub const SYSTEM_ADDRESS: &str = "0xffffFFFfFFffffffffffffffFfFFFfffFFFfFFfE";

/// System Contract manager
/// Handles ABI encoding/decoding and contract interactions
pub struct SystemContract {
    pub validator_abi: Value,
    pub stake_hub_abi: Value,
    pub slash_abi: Value,
    pub system_reward_abi: Value,
}

impl SystemContract {
    /// Create a new SystemContract instance
    pub fn new() -> Result<Self> {
        Ok(Self {
            validator_abi: abi::VALIDATOR_ABI.clone(),
            stake_hub_abi: abi::STAKE_HUB_ABI.clone(),
            slash_abi: abi::SLASH_ABI.clone(),
            system_reward_abi: abi::SYSTEM_REWARD_ABI.clone(),
        })
    }

    /// Get current validators (for pre-execution)
    /// Returns the contract address and calldata
    pub fn get_current_validators(&self) -> (String, Vec<u8>) {
        // Function: getMiningValidators()
        // This is a view function, doesn't modify state
        let function_selector = "0xda35c664"; // getMiningValidators()
        
        (
            VALIDATOR_CONTRACT.to_string(),
            hex::decode(&function_selector[2..]).unwrap(),
        )
    }

    /// Get validator election info from StakeHub
    /// Returns the contract address and calldata
    pub fn get_validator_election_info(&self, offset: u64, limit: u64) -> (String, Vec<u8>) {
        // Function: getValidatorElectionInfo(uint256,uint256)
        let function_selector = "0x96713da9";
        
        // Encode parameters (offset, limit)
        let mut calldata = hex::decode(&function_selector[2..]).unwrap();
        
        // Encode uint256 offset
        let offset_bytes = format!("{:064x}", offset);
        calldata.extend_from_slice(&hex::decode(&offset_bytes).unwrap());
        
        // Encode uint256 limit  
        let limit_bytes = format!("{:064x}", limit);
        calldata.extend_from_slice(&hex::decode(&limit_bytes).unwrap());
        
        (STAKE_HUB_CONTRACT.to_string(), calldata)
    }

    /// Decode validator set from contract response
    pub fn decode_validator_set(&self, _data: &[u8]) -> Result<Vec<ValidatorInfo>> {
        // This is a simplified version
        // In production, you'd use alloy-sol-types for proper ABI decoding
        
        // For now, return empty vector
        // TODO: Implement proper ABI decoding
        Ok(vec![])
    }

    /// Build deposit transaction data
    /// This is called by coinbase to distribute rewards
    pub fn build_deposit_call_data(&self, validator_addr: &str) -> Vec<u8> {
        // Function: deposit(address)
        let function_selector = "0xf340fa01";
        
        let mut calldata = hex::decode(&function_selector[2..]).unwrap();
        
        // Encode address parameter (remove 0x prefix and pad to 32 bytes)
        let addr_str = validator_addr.strip_prefix("0x").unwrap_or(validator_addr);
        let addr_bytes = format!("{:0>64}", addr_str);
        calldata.extend_from_slice(&hex::decode(&addr_bytes).unwrap());
        
        calldata
    }

    /// Build distribute finality reward transaction data
    pub fn build_distribute_finality_reward_call_data(
        &self,
        _validators: &[String],
        _weights: &[u64],
    ) -> Vec<u8> {
        // Function: distributeFinalityReward(address[],uint256[])
        let function_selector = "0x6e47b482";
        
        // TODO: Implement proper ABI encoding
        // For now, return just the selector
        hex::decode(&function_selector[2..]).unwrap()
    }

    /// Build updateValidatorSetV2 transaction data with empty arrays
    /// The contract will auto-fetch validator data from StakeHub
    pub fn build_update_validator_set_call_data(&self) -> Vec<u8> {
        // Function: updateValidatorSetV2(address[],uint64[],bytes[])
        // We pass empty arrays, and the contract will fetch data from StakeHub
        let function_selector = "0x056eae5d"; // Need to verify this selector
        
        // For now, encode with three empty arrays
        // TODO: Implement proper ABI encoding for empty arrays
        // Empty array encoding: 
        // - 0x60 (offset to first array) = 96 bytes
        // - 0xa0 (offset to second array) = 160 bytes  
        // - 0xe0 (offset to third array) = 224 bytes
        // - 0x00 (length of first array)
        // - 0x00 (length of second array)
        // - 0x00 (length of third array)
        
        let mut calldata = hex::decode(&function_selector[2..]).unwrap();
        
        // Offset to first array (after 3 offset values = 96 bytes)
        calldata.extend_from_slice(&hex::decode("0000000000000000000000000000000000000000000000000000000000000060").unwrap());
        // Offset to second array
        calldata.extend_from_slice(&hex::decode("00000000000000000000000000000000000000000000000000000000000000a0").unwrap());
        // Offset to third array
        calldata.extend_from_slice(&hex::decode("00000000000000000000000000000000000000000000000000000000000000e0").unwrap());
        // Length of first array (0)
        calldata.extend_from_slice(&hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap());
        // Length of second array (0)
        calldata.extend_from_slice(&hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap());
        // Length of third array (0)
        calldata.extend_from_slice(&hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap());
        
        calldata
    }
}

impl Default for SystemContract {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_system_contract() {
        let contract = SystemContract::new();
        assert!(contract.is_ok());
    }

    #[test]
    fn test_get_current_validators() {
        let contract = SystemContract::new().unwrap();
        let (addr, data) = contract.get_current_validators();
        
        assert_eq!(addr, VALIDATOR_CONTRACT);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_build_deposit_call_data() {
        let contract = SystemContract::new().unwrap();
        let validator = "0x1234567890123456789012345678901234567890";
        let data = contract.build_deposit_call_data(validator);
        
        // Should have function selector (4 bytes) + address (32 bytes)
        assert_eq!(data.len(), 36);
    }
}

