//! The Application (or Node) definition. The Node trait implements the Consensus context and the
//! cryptographic library used for signing.

use std::path::PathBuf;
use std::str::FromStr;

use async_trait::async_trait;
use color_eyre::eyre;
use malachitebft_app_channel::app::events::{RxEvent, TxEvent};
use malachitebft_app_channel::app::node::{
    CanGeneratePrivateKey, CanMakeConfig, CanMakeGenesis, CanMakePrivateKeyFile, EngineHandle,
    MakeConfigSettings, Node, NodeHandle,
};
use malachitebft_eth_engine::engine::Engine;
use malachitebft_eth_engine::engine_rpc::EngineRPC;
use malachitebft_eth_engine::ethereum_rpc::EthereumRPC;
use rand::{CryptoRng, RngCore};
use tracing::info;

use malachitebft_app_channel::app::metrics::SharedRegistry;
use malachitebft_app_channel::app::types::core::VotingPower;
use malachitebft_app_channel::app::types::Keypair;

use crate::app_config::{load_config, Config};
use crate::metrics::DbMetrics;
use crate::state::State;
use crate::store::Store;
use malachitebft_eth_cli::metrics;
use malachitebft_eth_types::codec::proto::ProtobufCodec;
use malachitebft_eth_types::{
    Address, Ed25519Provider, Genesis, Height, PrivateKey, PublicKey, TestContext, Validator,
    ValidatorSet,
};
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::mpsc,
    task::JoinHandle,
};
use tracing::Instrument;
use url::Url;

/// Main application struct implementing the consensus node functionality
#[derive(Clone)]
pub struct App {
    pub config_file: PathBuf,
    pub home_dir: PathBuf,
    pub genesis_file: PathBuf,
    pub private_key_file: PathBuf,
    pub start_height: Option<Height>,
}

pub struct Handle {
    pub app: JoinHandle<()>,
    pub engine: EngineHandle,
    pub tx_event: TxEvent<TestContext>,
}

#[async_trait]
impl NodeHandle<TestContext> for Handle {
    fn subscribe(&self) -> RxEvent<TestContext> {
        self.tx_event.subscribe()
    }

    async fn kill(&self, _reason: Option<String>) -> eyre::Result<()> {
        self.engine.actor.kill_and_wait(None).await?;
        self.app.abort();
        self.engine.handle.abort();
        Ok(())
    }
}

#[async_trait]
impl Node for App {
    type Context = TestContext;
    type Genesis = Genesis;
    type PrivateKeyFile = PrivateKey;
    type SigningProvider = Ed25519Provider;
    type NodeHandle = Handle;
    type Config = Config;

    fn get_home_dir(&self) -> PathBuf {
        self.home_dir.to_owned()
    }

    fn get_signing_provider(&self, private_key: PrivateKey) -> Self::SigningProvider {
        Ed25519Provider::new(private_key)
    }

    fn load_config(&self) -> eyre::Result<Self::Config> {
        load_config(&self.config_file, Some("MALACHITE"))
    }

    fn get_address(&self, pk: &PublicKey) -> Address {
        Address::from_public_key(pk)
    }

    fn get_public_key(&self, pk: &PrivateKey) -> PublicKey {
        pk.public_key()
    }

    fn get_keypair(&self, pk: PrivateKey) -> Keypair {
        Keypair::ed25519_from_bytes(pk.inner().to_bytes()).unwrap()
    }

    fn load_private_key(&self, file: Self::PrivateKeyFile) -> PrivateKey {
        file
    }

    fn load_private_key_file(&self) -> eyre::Result<Self::PrivateKeyFile> {
        let private_key = std::fs::read_to_string(&self.private_key_file)?;
        serde_json::from_str(&private_key).map_err(|e| e.into())
    }

    fn load_genesis(&self) -> eyre::Result<Self::Genesis> {
        let genesis = std::fs::read_to_string(&self.genesis_file)?;
        serde_json::from_str(&genesis).map_err(|e| e.into())
    }

    async fn start(&self) -> eyre::Result<Self::NodeHandle> {
        // This is required by the Node trait
        // But we'll use run() for the actual implementation
        unimplemented!("Use run() instead")
    }

    async fn run(self) -> eyre::Result<()> {
        let config = self.load_config()?;

        let private_key_file = self.load_private_key_file()?;
        let private_key = self.load_private_key(private_key_file);
        let public_key = self.get_public_key(&private_key);
        let address = self.get_address(&public_key);
        let signing_provider = self.get_signing_provider(private_key);
        let ctx = TestContext::new();

        // NEW: Load initial data from Reth's genesis extraData
        // No longer need separate Malachite genesis.json!
        info!("📖 Loading initial data from Reth genesis extraData...");
        let (initial_validator_set, epoch_length) =
            self.load_initial_data_from_reth(&config).await?;
        info!(
            "✅ Initial data loaded: {} validators, epoch_length: {} blocks",
            initial_validator_set.validators.len(),
            epoch_length
        );

        let (mut channels, engine_handle) = malachitebft_app_channel::start_engine(
            ctx.clone(),
            self.clone(),
            config.clone(),
            ProtobufCodec,
            ProtobufCodec,
            self.start_height,
            initial_validator_set.clone(),
        )
        .await?;

        let _tx_event = channels.events.clone();

        let registry = SharedRegistry::global().with_moniker(&config.moniker);
        let metrics = DbMetrics::register(&registry);

        if config.metrics.enabled {
            tokio::spawn(metrics::serve(config.metrics.listen_addr));
        }

        let db_dir = self.get_home_dir().join("db");
        std::fs::create_dir_all(&db_dir)?;

        let store = Store::open(self.get_home_dir().join("store.db"), metrics)?;
        let start_height = self.start_height.unwrap_or_default();

        let mut state = State::new(
            initial_validator_set,
            epoch_length,
            ctx,
            signing_provider,
            address,
            start_height,
            store,
            config.prune.clone(),
        );

        let engine: Engine = {
            let engine_url: Url = {
                let url = config.engine.engine_url.as_str();
                Url::parse(url)?
            };
            let jwt_path = PathBuf::from_str(config.engine.wt_path.as_str())?;
            let eth_url: Url = {
                let url = config.engine.eth_url.as_str();
                Url::parse(&url)?
            };
            Engine::new(
                EngineRPC::new(engine_url, jwt_path.as_path())?,
                EthereumRPC::new(eth_url)?,
            )
        };

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        let _ = tokio::spawn(async move {
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM signal, shutting down gracefully...");
                }
                _ = sigint.recv() => {
                    tracing::info!("Received SIGINT signal, shutting down gracefully...");
                }
            }

            let _ = shutdown_tx.send(()).await;
        });

        let span = tracing::error_span!("node", moniker = %config.moniker);
        crate::app::run(
            &mut state,
            &mut channels,
            engine,
            config.engine.block_interval,
            shutdown_rx,
        )
        .instrument(span)
        .await?;

        engine_handle.actor.kill_and_wait(None).await?;

        Ok(())
    }
}

impl App {
    /// NEW: Load initial data from Reth's genesis block extraData
    /// This eliminates the need for a separate Malachite genesis.json file
    async fn load_initial_data_from_reth(
        &self,
        config: &Config,
    ) -> eyre::Result<(ValidatorSet, u64)> {
        use malachitebft_eth_engine::genesis::parse_validators_from_extra_data;
        use malachitebft_eth_types::PublicKey;
        use malachitebft_eth_types::Validator;
        use url::Url;

        // Step 1: Get Reth RPC URL from config
        let eth_url_str = config.engine.eth_url.clone();
        let eth_url = Url::parse(&eth_url_str)?;
        let eth_rpc = EthereumRPC::new(eth_url)?;

        info!(
            "📡 Connecting to Reth at {} to fetch genesis block...",
            eth_url_str
        );

        // Step 2: Get genesis block from Reth
        let genesis_block = eth_rpc
            .get_block_by_number("0x0")
            .await?
            .ok_or_else(|| eyre::eyre!("Genesis block not found in Reth"))?;

        info!("✅ Got genesis block from Reth");
        info!("   Block hash: {}", genesis_block.block_hash);
        info!(
            "   ExtraData length: {} bytes",
            genesis_block.extra_data.len()
        );
        info!(
            "   ExtraData hex: 0x{}...",
            hex::encode(
                &genesis_block.extra_data[..std::cmp::min(32, genesis_block.extra_data.len())]
            )
        );

        // Step 3: Parse extraData to get validators with Tendermint public keys
        // Use bytes directly instead of converting to hex string
        let (validator_infos, epoch_length) =
            parse_validators_from_extra_data(&genesis_block.extra_data)?;

        info!(
            "✅ Parsed {} validators from extended extraData format, epoch_length: {} blocks",
            validator_infos.len(),
            epoch_length
        );

        // Step 4: Convert to Malachite ValidatorSet
        let validators: Vec<Validator> = validator_infos
            .into_iter()
            .enumerate()
            .map(|(_i, info)| {
                // Validate Tendermint public key length
                if info.tendermint_pubkey.len() != 32 {
                    return Err(eyre::eyre!(
                        "Invalid Tendermint public key length for validator {}: {} bytes",
                        info.consensus_address,
                        info.tendermint_pubkey.len()
                    ));
                }

                // Check if it's a placeholder (all zeros) - reject it
                let is_placeholder = info.tendermint_pubkey.iter().all(|&b| b == 0);
                if is_placeholder {
                    return Err(eyre::eyre!(
                        "Validator {} has invalid Tendermint public key (all zeros). \
                         Please add real Tendermint public keys to validators.js. \
                         You can derive them from priv_validator_key.json",
                        info.consensus_address
                    ));
                }

                // Convert Tendermint public key to array
                let pubkey_array: [u8; 32] = info
                    .tendermint_pubkey
                    .try_into()
                    .map_err(|_| eyre::eyre!("Invalid public key length"))?;

                // Convert to Malachite PublicKey
                let public_key = PublicKey::from_bytes(pubkey_array);

                Ok(Validator::new(
                    public_key,
                    VotingPower::from(info.voting_power),
                ))
            })
            .collect::<Result<Vec<_>, eyre::Report>>()?;

        if validators.is_empty() {
            return Err(eyre::eyre!("No validators found in genesis extraData"));
        }

        info!(
            "🎉 Successfully built initial validator set with {} validators",
            validators.len()
        );

        Ok((ValidatorSet::new(validators), epoch_length))
    }
}

impl CanMakeGenesis for App {
    fn make_genesis(&self, validators: Vec<(PublicKey, VotingPower)>) -> Self::Genesis {
        let validators = validators
            .into_iter()
            .map(|(pk, vp)| Validator::new(pk, vp));

        let validator_set = ValidatorSet::new(validators);

        Genesis { validator_set }
    }
}

impl CanGeneratePrivateKey for App {
    fn generate_private_key<R>(&self, rng: R) -> PrivateKey
    where
        R: RngCore + CryptoRng,
    {
        PrivateKey::generate(rng)
    }
}

impl CanMakePrivateKeyFile for App {
    fn make_private_key_file(&self, private_key: PrivateKey) -> Self::PrivateKeyFile {
        private_key
    }
}

impl CanMakeConfig for App {
    fn make_config(index: usize, total: usize, settings: MakeConfigSettings) -> Self::Config {
        make_config(index, total, settings)
    }
}

/// Generate configuration for node "index" out of "total" number of nodes.
fn make_config(index: usize, total: usize, settings: MakeConfigSettings) -> Config {
    use itertools::Itertools;
    use rand::seq::IteratorRandom;
    use rand::Rng;

    use malachitebft_app_channel::app::config::*;

    const CONSENSUS_BASE_PORT: usize = 27000;
    const METRICS_BASE_PORT: usize = 29000;

    let consensus_port = CONSENSUS_BASE_PORT + index;
    let metrics_port = METRICS_BASE_PORT + index;

    Config {
        moniker: format!("app-{}", index),
        consensus: ConsensusConfig {
            // Current channel app does not support parts-only value payload properly as Init does not include valid_round
            value_payload: ValuePayload::ProposalAndParts,
            queue_capacity: 100,
            timeouts: TimeoutConfig::default(),
            p2p: P2pConfig {
                protocol: PubSubProtocol::default(),
                listen_addr: settings.transport.multiaddr("127.0.0.1", consensus_port),
                persistent_peers: if settings.discovery.enabled {
                    let mut rng = rand::thread_rng();
                    let count = if total > 1 {
                        rng.gen_range(1..=(total / 2))
                    } else {
                        0
                    };
                    let peers = (0..total)
                        .filter(|j| *j != index)
                        .choose_multiple(&mut rng, count);

                    peers
                        .iter()
                        .unique()
                        .map(|index| {
                            settings
                                .transport
                                .multiaddr("127.0.0.1", CONSENSUS_BASE_PORT + index)
                        })
                        .collect()
                } else {
                    (0..total)
                        .filter(|j| *j != index)
                        .map(|j| {
                            settings
                                .transport
                                .multiaddr("127.0.0.1", CONSENSUS_BASE_PORT + j)
                        })
                        .collect()
                },
                discovery: settings.discovery,
                ..Default::default()
            },
        },
        metrics: MetricsConfig {
            enabled: true,
            listen_addr: format!("127.0.0.1:{metrics_port}").parse().unwrap(),
        },
        runtime: settings.runtime,
        logging: LoggingConfig::default(),
        value_sync: ValueSyncConfig::default(),
        engine: Default::default(),
        prune: Default::default(),
    }
}
