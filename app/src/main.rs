//! Example application using channels

use color_eyre::eyre::{eyre, Result};
use tracing::{info, trace};

use malachitebft_app_channel::app::node::Node;
use malachitebft_config::*;
use malachitebft_eth_cli::args::{Args, Commands};
use malachitebft_eth_cli::cmd::init::InitCmd;
use malachitebft_eth_cli::cmd::start::StartCmd;
use malachitebft_eth_cli::cmd::testnet::TestnetCmd;
use malachitebft_eth_cli::{logging, runtime};
use malachitebft_test::{Genesis, Height, PrivateKey, ValidatorSet, Validator};

mod app;
mod app_config;
mod metrics;
mod node;
mod state;
mod store;
mod streaming;

use node::App;

/// Main entry point for the application
///
/// This function:
/// - Parses command-line arguments
/// - Loads configuration from file
/// - Initializes logging system
/// - Sets up error handling
/// - Creates and runs the application node
fn main() -> Result<()> {
    color_eyre::install()?;

    // Load command-line arguments and possible configuration file.
    let args = Args::new();

    // Override logging configuration (if exists) with optional command-line parameters.
    let mut logging = LoggingConfig::default();
    if let Some(log_level) = args.log_level {
        logging.log_level = log_level;
    }
    if let Some(log_format) = args.log_format {
        logging.log_format = log_format;
    }

    // This is a drop guard responsible for flushing any remaining logs when the program terminates.
    // It must be assigned to a binding that is not _, as _ will result in the guard being dropped immediately.
    let _guard = logging::init(logging.log_level, logging.log_format);

    trace!("Command-line parameters: {args:?}");

    // Parse the input command.
    match &args.command {
        Commands::Start(cmd) => start(&args, cmd),
        Commands::Init(cmd) => init(&args, cmd),
        Commands::Testnet(cmd) => testnet(&args, cmd),
        _ => unimplemented!(),
    }
}

fn start(args: &Args, cmd: &StartCmd) -> Result<()> {
    // Load configuration file if it exists. Some commands do not require a configuration file.
    let config_file = args
        .get_config_file_path()
        .map_err(|error| eyre!("Failed to get configuration file path: {error}"))?;

    let config = app_config::load_config(&config_file, None)
        .map_err(|error| eyre!("Failed to load configuration file: {error}"))?;

    let rt = runtime::build_runtime(config.runtime)?;

    info!(
        file = %args.get_config_file_path().unwrap_or_default().display(),
        "Loaded configuration",
    );

    trace!(?config, "Configuration");

    // Load genesis file
    let genesis_file = args.get_genesis_file_path()?;
    let genesis_content = std::fs::read_to_string(&genesis_file)
        .map_err(|error| eyre!("Failed to read genesis file: {error}"))?;
    let genesis: Genesis = serde_json::from_str(&genesis_content)
        .map_err(|error| eyre!("Failed to parse genesis file: {error}"))?;

    // Load private key file
    let private_key_file = args.get_priv_validator_key_file_path()?;
    let private_key_content = std::fs::read_to_string(&private_key_file)
        .map_err(|error| eyre!("Failed to read private key file: {error}"))?;
    let private_key: PrivateKey = serde_json::from_str(&private_key_content)
        .map_err(|error| eyre!("Failed to parse private key file: {error}"))?;

    // Setup the application
    let app = App {
        home_dir: args.get_home_dir()?,
        config,
        validator_set: genesis.validator_set.clone(),
        private_key,
        start_height: cmd.start_height.map(Height::new),
        middleware: None,
    };

    // Start the node
    rt.block_on(app.run())
        .map_err(|error| eyre!("Failed to run the application node: {error}"))
}

fn init(args: &Args, cmd: &InitCmd) -> Result<()> {
    // Generate a dummy private key for the validator
    let dummy_private_key = PrivateKey::generate(rand::thread_rng());
    let dummy_public_key = dummy_private_key.public_key();
    
    // Create a dummy validator with voting power 1
    let dummy_validator = Validator::new(dummy_public_key, 1);
    
    // Setup the application with minimal data for init command
    let app = App {
        home_dir: args.get_home_dir()?,
        config: Default::default(),
        validator_set: ValidatorSet::new(vec![dummy_validator]), // One dummy validator
        private_key: PrivateKey::generate(rand::thread_rng()),
        start_height: Some(Height::new(1)), // We always start at height 1
        middleware: None,
    };

    cmd.run(
        &app,
        &args.get_config_file_path()?,
        &args.get_genesis_file_path()?,
        &args.get_priv_validator_key_file_path()?,
    )
    .map_err(|error| eyre!("Failed to run init command {error:?}"))
}

fn testnet(args: &Args, cmd: &TestnetCmd) -> Result<()> {
    println!("testnet");
    // Generate a dummy private key for the validator
    let dummy_private_key = PrivateKey::generate(rand::thread_rng());
    let dummy_public_key = dummy_private_key.public_key();
    
    // Create a dummy validator with voting power 1
    let dummy_validator = Validator::new(dummy_public_key, 1);
    
    // Setup the application with minimal data for testnet command
    let app = App {
        home_dir: args.get_home_dir()?,
        config: Default::default(),
        validator_set: ValidatorSet::new(vec![dummy_validator]), // One dummy validator
        private_key: PrivateKey::generate(rand::thread_rng()),
        start_height: Some(Height::new(1)), // We always start at height 1
        middleware: None,
    };
    println!("testnet2");
    cmd.run(&app, &args.get_home_dir()?)
        .map_err(|error| eyre!("Failed to run testnet command {:?}", error))
}
