// ABI loading module
// Loads contract ABIs from JSON files

use once_cell::sync::Lazy;
use serde_json::Value;

/// ValidatorSet contract ABI
pub static VALIDATOR_ABI: Lazy<Value> = Lazy::new(|| {
    let abi_json = include_str!("abis/ValidatorSet.json");
    serde_json::from_str(abi_json).expect("Failed to parse ValidatorSet ABI")
});

/// StakeHub contract ABI
pub static STAKE_HUB_ABI: Lazy<Value> = Lazy::new(|| {
    let abi_json = include_str!("abis/StakeHub.json");
    serde_json::from_str(abi_json).expect("Failed to parse StakeHub ABI")
});

/// SlashIndicator contract ABI
pub static SLASH_ABI: Lazy<Value> = Lazy::new(|| {
    let abi_json = include_str!("abis/SlashIndicator.json");
    serde_json::from_str(abi_json).expect("Failed to parse SlashIndicator ABI")
});

/// SystemReward contract ABI
pub static SYSTEM_REWARD_ABI: Lazy<Value> = Lazy::new(|| {
    let abi_json = include_str!("abis/SystemReward.json");
    serde_json::from_str(abi_json).expect("Failed to parse SystemReward ABI")
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_validator_abi() {
        let abi = &*VALIDATOR_ABI;
        assert!(abi.is_array());
        assert!(!abi.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_load_stake_hub_abi() {
        let abi = &*STAKE_HUB_ABI;
        assert!(abi.is_array());
        assert!(!abi.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_load_slash_abi() {
        let abi = &*SLASH_ABI;
        assert!(abi.is_array());
        assert!(!abi.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_load_system_reward_abi() {
        let abi = &*SYSTEM_REWARD_ABI;
        assert!(abi.is_array());
        assert!(!abi.as_array().unwrap().is_empty());
    }
}

