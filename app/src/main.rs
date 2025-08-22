//! Example application using channels

use color_eyre::eyre::{eyre, Result};
use tracing::{info, trace};

use malachitebft_app_channel::app::node::Node;
use malachitebft_eth_cli::args::{Args, Commands};
use malachitebft_eth_cli::cmd::init::InitCmd;
use malachitebft_eth_cli::cmd::start::StartCmd;
use malachitebft_eth_cli::cmd::testnet::TestnetCmd;
use malachitebft_eth_cli::{runtime};
use malachitebft_eth_types::Height;

mod app;
mod metrics;
mod node;
mod state;
mod store;
mod streaming;
mod app_config;

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

    // Setup the application
    let app = App {
        config_file,
        home_dir: args.get_home_dir()?,
        genesis_file: args.get_genesis_file_path()?,
        private_key_file: args.get_priv_validator_key_file_path()?,
        start_height: cmd.start_height.map(Height::new),
    };

    // Start the node
    rt.block_on(app.run())
        .map_err(|error| eyre!("Failed to run the application node: {error}"))
}

fn init(args: &Args, cmd: &InitCmd) -> Result<()> {
    // Setup the application
    let app = App {
        config_file: Default::default(),
        home_dir: args.get_home_dir()?,
        genesis_file: args.get_genesis_file_path()?,
        private_key_file: args.get_priv_validator_key_file_path()?,
        start_height: Some(Height::new(1)), // We always start at height 1
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
    // Setup the application
    let app = App {
        config_file: Default::default(),
        home_dir: args.get_home_dir()?,
        genesis_file: args.get_genesis_file_path()?,
        private_key_file: args.get_priv_validator_key_file_path()?,
        start_height: Some(Height::new(1)), // We always start at height 1
    };

    cmd.run(&app, &args.get_home_dir()?)
        .map_err(|error| eyre!("Failed to run testnet command {:?}", error))
}
