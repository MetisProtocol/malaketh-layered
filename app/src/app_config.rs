use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use malachitebft_config::*;
use malachitebft_app_channel::app::node::NodeConfig;

/// Prune configuration options
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PruneConfig {
    /// Whether to enable automatic pruning of old data
    pub enabled: bool,
    /// Number of heights to retain (keep the last N heights)
    pub retain_heights: u64,
}

impl Default for PruneConfig {
    fn default() -> Self {
        PruneConfig {
            enabled: true,
            retain_heights: 5000,
        }
    }
}

/// Dynamic Validator Set configuration options
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DynamicValidatorSetConfig {
    /// Whether dynamic validator set is enabled
    pub enabled: bool,
    /// Contract address for ValidatorSetManager
    pub contract_address: Option<String>,
    /// Update interval in seconds
    pub update_interval_seconds: u64,
    /// Epoch length in blocks
    pub epoch_length_blocks: u64,
    /// Minimum stake amount in wei
    pub min_stake_amount: String,
    /// Whether slashing is enabled
    pub slashing_enabled: bool,
    /// Whether fee distribution is enabled
    pub fee_distribution_enabled: bool,
}

impl Default for DynamicValidatorSetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            contract_address: None,
            update_interval_seconds: 30,
            epoch_length_blocks: 100,
            min_stake_amount: "1000000000000000000".to_string(), // 1 ETH
            slashing_enabled: true,
            fee_distribution_enabled: true,
        }
    }
}

impl DynamicValidatorSetConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.contract_address.is_none() {
            return Err("contract_address is required when dynamic validator set is enabled".to_string());
        }
        if self.epoch_length_blocks == 0 {
            return Err("epoch_length_blocks must be greater than 0".to_string());
        }
        if self.update_interval_seconds == 0 {
            return Err("update_interval_seconds must be greater than 0".to_string());
        }
        if let Some(addr) = &self.contract_address {
            if !addr.starts_with("0x") || addr.len() != 42 {
                return Err(format!("Invalid contract address format: {}", addr));
            }
        }
        Ok(())
    }
}

/// Extra Malachite configuration options
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineConfig {
    /// engine_url, ip:port
    pub engine_url: String,
    /// eth_url, ip:port
    pub eth_url: String,
    /// path of jwtsecret
    pub wt_path: String,
    /// block interval time
    #[serde(with = "humantime_serde")]
    pub block_interval: Duration,
    /// Dynamic validator set configuration
    pub dynamic_validator_set: DynamicValidatorSetConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            engine_url: "http://localhost:8551".to_string(),
            eth_url: "http://localhost:8545".to_string(),
            wt_path: "./assets/jwtsecret".to_string(),
            block_interval: Duration::from_millis(1000),
            dynamic_validator_set: DynamicValidatorSetConfig::default(),
        }
    }
}

/// load_config parses the environment variables and loads the provided config file path
/// to create a Config struct.
pub fn load_config(path: impl AsRef<Path>, prefix: Option<&str>) -> eyre::Result<Config> {
    ::config::Config::builder()
        .add_source(::config::File::from(path.as_ref()))
        .add_source(
            ::config::Environment::with_prefix(prefix.unwrap_or("MALACHITE")).separator("__"),
        )
        .build()?
        .try_deserialize()
        .map_err(Into::into)
}

/// Malachite configuration options
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// A custom human-readable name for this node
    pub moniker: String,

    /// Engine configuration options
    pub engine: EngineConfig,

    /// Log configuration options
    pub logging: LoggingConfig,

    /// Consensus configuration options
    pub consensus: ConsensusConfig,

    /// ValueSync configuration options
    pub value_sync: ValueSyncConfig,

    /// Metrics configuration options
    pub metrics: MetricsConfig,

    /// Runtime configuration options
    pub runtime: RuntimeConfig,

    /// Prune configuration options
    pub prune: PruneConfig,
}

impl NodeConfig for Config {
    fn moniker(&self) -> &str {
        &self.moniker
    }

    fn consensus(&self) -> &ConsensusConfig {
        &self.consensus
    }

    fn value_sync(&self) -> &ValueSyncConfig {
        &self.value_sync
    }
}
