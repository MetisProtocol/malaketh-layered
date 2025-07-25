use serde::{Deserialize, Serialize};
use std::path::Path;
use malachitebft_config::*;
use malachitebft_app_channel::app::node::NodeConfig;

/// Extra Malachite configuration options
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineConfig {
    /// engine_url, ip:port
    pub engine_url: String,
    /// eth_url, ip:port
    pub eth_url: String,
    /// path of jwtsecret
    pub wt_path: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            engine_url: "http://localhost:8551".to_string(),
            eth_url: "http://localhost:8545".to_string(),
            wt_path: "./configs/assets/jwtsecret".to_string(),
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