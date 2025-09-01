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
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            engine_url: "http://localhost:8551".to_string(),
            eth_url: "http://localhost:8545".to_string(),
            wt_path: "./assets/jwtsecret".to_string(),
            block_interval: Duration::from_millis(1000),
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
