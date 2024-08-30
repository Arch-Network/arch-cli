use bitcoin::Amount;
use bitcoin::Network;
use bitcoincore_rpc::json::GetTransactionResult;
use clap::{ Parser, Subcommand, Args };
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tokio;
use anyhow::{ Context, Result };
use std::process::Command as ShellCommand;
use common::helper::*;
use common::constants::*;
use bitcoin::{ Address, PublicKey };
use bitcoincore_rpc::{ Auth, Client, RawTx, RpcApi };
use std::time::Duration;
use std::str::FromStr;
use colored::*;
use std::process::Command;
use config::{ Config, File, Environment };
use std::env;
mod docker_manager;
use anyhow::anyhow;

#[derive(Deserialize)]
struct ServiceConfig {
    docker_compose_file: String,
    services: Vec<String>,
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    StartServer,
    Deploy(DeployArgs),
    StopServer,
    Clean,
}

#[derive(Args)]
struct DeployArgs {
    #[clap(long, help = "Directory of your program")]
    directory: Option<String>,

    #[clap(long, help = "Path to the program key file")]
    program_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let config = Config::builder()
        .add_source(File::with_name("config.toml"))
        .add_source(Environment::default())
        .build()
        .context("Failed to load configuration")?;

    match &cli.command {
        Commands::Init => init().await,
        Commands::StartServer => start_server(&config).await,
        Commands::Deploy(args) => deploy(args, &config).await,
        Commands::StopServer => stop_server(&config).await,
        Commands::Clean => clean().await,
    }
}
async fn init() -> Result<()> {
    println!("{}", "Initializing new Arch Network app...".bold().green());

    // Navigate to the program folder and run `cargo build-sbf`
    println!("Building Arch Network program...");
    ShellCommand::new("cargo")
        .current_dir("program")
        .arg("build-sbf")
        .output()
        .expect("Failed to build Arch Network program");

    // Create project structure
    println!("Creating project structure...");
    let dirs = ["src/app/program/src", "src/app/backend", "src/app/frontend", "src/app/keys"];

    for dir in dirs.iter() {
        fs::create_dir_all(dir).with_context(|| format!("Failed to create directory: {}", dir))?;
    }

    // Create boilerplate files
    println!("Creating boilerplate files...");
    let files = [
        ("src/app/program/src/lib.rs", include_str!("templates/program_lib.rs")),
        ("src/app/program/Cargo.toml", include_str!("templates/program_cargo.toml")),
        ("src/app/backend/index.ts", include_str!("templates/backend_index.ts")),
        ("src/app/backend/package.json", include_str!("templates/backend_package.json")),
        ("src/app/frontend/index.html", include_str!("templates/frontend_index.html")),
        ("src/app/frontend/index.js", include_str!("templates/frontend_index.js")),
        ("src/app/frontend/package.json", include_str!("templates/frontend_package.json")),
    ];

    for (file_path, content) in files.iter() {
        fs
            ::write(file_path, content)
            .with_context(|| format!("Failed to write file: {}", file_path))?;
    }

    println!("  {} New Arch Network app initialized successfully!", "✓".bold().green());
    Ok(())
}

fn start_or_create_services(service_name: &str, service_config: &ServiceConfig) -> Result<()> {
    println!("Starting {}...", service_name);

    let mut all_containers_exist = true;
    let mut all_containers_running = true;

    for container in &service_config.services {
        let ps_output = Command::new("docker-compose")
            .args(&["-f", &service_config.docker_compose_file, "ps", "-q", container])
            .output()
            .context(format!("Failed to check existing container for {}", container))?;

        if ps_output.stdout.is_empty() {
            all_containers_exist = false;
            all_containers_running = false;
            break;
        }

        let status_output = Command::new("docker")
            .args(
                &[
                    "inspect",
                    "-f",
                    "{{.State.Running}}",
                    &String::from_utf8_lossy(&ps_output.stdout).trim(),
                ]
            )
            .output()
            .context(format!("Failed to check status of container {}", container))?;

        if String::from_utf8_lossy(&status_output.stdout).trim() != "true" {
            all_containers_running = false;
        }
    }

    if all_containers_exist {
        if all_containers_running {
            println!("All {} containers are already running.", service_name);
        } else {
            println!("Existing {} containers found. Starting them...", service_name);
            let start_output = Command::new("docker-compose")
                .args(&["-f", &service_config.docker_compose_file, "start"])
                .output()
                .context(format!("Failed to start existing {} containers", service_name))?;

            if !start_output.status.success() {
                let error_message = String::from_utf8_lossy(&start_output.stderr);
                println!(
                    "Warning: Failed to start some {} containers: {}",
                    service_name,
                    error_message
                );
            } else {
                println!("{} containers started successfully.", service_name);
            }
        }
    } else {
        println!("Some or all {} containers are missing. Creating and starting new ones...", service_name);
        let up_output = Command::new("docker-compose")
            .args(&["-f", &service_config.docker_compose_file, "up", "-d"])
            .output()
            .context(format!("Failed to create and start {} containers", service_name))?;

        if !up_output.status.success() {
            let error_message = String::from_utf8_lossy(&up_output.stderr);
            println!(
                "Warning: Failed to create and start {} containers: {}",
                service_name,
                error_message
            );
        } else {
            println!("{} containers created and started successfully.", service_name);
        }
    }

    Ok(())
}
async fn start_server(config: &Config) -> Result<()> {
    println!("{}", "Starting development server...".bold().green());

    let network_type = config
        .get_string("network.type")
        .context("Failed to get network type from configuration")?;

    if network_type == "development" {
        set_env_vars(config)?;
        create_docker_network("arch-network")?;

        let bitcoin_config: ServiceConfig = config
            .get("bitcoin")
            .context("Failed to get Bitcoin configuration")?;
        start_or_create_services("Bitcoin regtest network", &bitcoin_config)?;

        let arch_config: ServiceConfig = config
            .get("arch")
            .context("Failed to get Arch Network configuration")?;
        start_or_create_services("Arch Network nodes", &arch_config)?;
    } else {
        println!("Using existing network configuration for: {}", network_type);
    }

    println!("  {} Development server started successfully!", "✓".bold().green());

    Ok(())
}

async fn deploy(args: &DeployArgs, config: &Config) -> Result<()> {
    println!("{}", "Deploying your Arch Network app...".bold().green());

    // Build the program
    build_program(args)?;

    // Get program key and public key
    let program_key_path = get_program_key_path(args, config)?;
    let (program_keypair, program_pubkey) = with_secret_key_file(&program_key_path).context(
        "Failed to get program key pair"
    )?;

    // Get account address
    let account_address = get_account_address_async(program_pubkey).await.context(
        "Failed to get account address"
    )?;

    println!("  {} Program account created", "✓".bold().green());
    println!("  {} Account address: {}", "ℹ".bold().blue(), account_address.yellow());

    // Set up Bitcoin RPC client
    let rpc = setup_bitcoin_rpc_client(config)?;

    // Handle fund transfer based on network type
    let tx_info = handle_fund_transfer(&rpc, &account_address, config).await?;

    // Deploy the program
    deploy_program_with_tx_info(&program_keypair, &program_pubkey, tx_info).await?;

    println!("{}", "Your app has been deployed successfully!".bold().green());
    Ok(())
}
async fn stop_server(config: &Config) -> Result<()> {
    println!("{}", "Stopping development server...".bold().yellow());

    // Stop all containers related to our development environment
    stop_all_related_containers()?;

    println!("{}", "Development server stopped successfully!".bold().green());
    println!("{}", "You can restart the server later using the 'start-server' command.".italic());
    Ok(())
}

fn stop_all_related_containers() -> Result<()> {
    let container_prefixes = vec!["arch-cli", "bitcoin", "electrs", "btc-rpc-explorer"];

    for prefix in container_prefixes {
        println!("Stopping {} containers...", prefix);

        // List all running containers with the given prefix
        let output = Command::new("docker")
            .args(&["ps", "-q", "--filter", &format!("name={}", prefix)])
            .output()
            .context(format!("Failed to list running {} containers", prefix))?;

        let container_ids = String::from_utf8_lossy(&output.stdout);

        if !container_ids.is_empty() {
            // Stop the containers
            let stop_output = Command::new("docker")
                .arg("stop")
                .args(container_ids.split_whitespace())
                .output()
                .context(format!("Failed to stop {} containers", prefix))?;

            if !stop_output.status.success() {
                let error_message = String::from_utf8_lossy(&stop_output.stderr);
                println!("Warning: Failed to stop some {} containers: {}", prefix, error_message);
            } else {
                println!("{} containers stopped successfully.", prefix);
            }
        } else {
            println!("No running {} containers found to stop.", prefix);
        }
    }

    Ok(())
}

fn start_existing_containers(compose_file: &str) -> Result<()> {
    let output = Command::new("docker-compose")
        .args(&["-f", compose_file, "ps", "-q"])
        .output()
        .context("Failed to list existing containers")?;

    if !output.stdout.is_empty() {
        println!("Found existing containers. Starting them...");
        let start_output = Command::new("docker-compose")
            .args(&["-f", compose_file, "start"])
            .output()
            .context("Failed to start existing containers")?;

        if !start_output.status.success() {
            let error_message = String::from_utf8_lossy(&start_output.stderr);
            println!("Warning: Failed to start some containers: {}", error_message);
        } else {
            println!("Existing containers started successfully.");
        }
    } else {
        println!("No existing containers found. Creating new ones...");
        // Proceed with your existing logic to create new containers
    }

    Ok(())
}

fn remove_docker_networks() -> Result<()> {
    let networks = vec!["arch-network", "internal"];

    for network in networks {
        println!("Removing Docker network: {}", network);

        let output = Command::new("docker")
            .args(&["network", "rm", network])
            .output()
            .context(format!("Failed to remove Docker network: {}", network))?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            if error_message.contains("not found") {
                println!("Network {} not found. Skipping.", network);
            } else {
                println!("Warning: Failed to remove network {}: {}", network, error_message);
            }
        } else {
            println!("Network {} removed successfully.", network);
        }
    }

    Ok(())
}
fn stop_docker_services(compose_file: &str, service_name: &str) -> Result<()> {
    println!("Stopping {} services...", service_name);
    let output = Command::new("docker-compose")
        .args(&["-f", compose_file, "down"])
        .output()
        .context(format!("Failed to stop {} services", service_name))?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        println!("Warning: Failed to stop {} services: {}", service_name, error_message);
    } else {
        println!("{} services stopped successfully.", service_name);
    }

    Ok(())
}

async fn clean() -> Result<()> {
    println!("Cleaning project...");

    // Remove src/app directory
    fs::remove_dir_all("src/app")?;

    println!("Project cleaned successfully!");
    Ok(())
}

fn start_bitcoin_regtest() -> Result<()> {
    println!("Starting Bitcoin regtest network...");
    Command::new("docker-compose")
        .arg("-f")
        .arg("path/to/bitcoin-docker-compose.yml")
        .arg("up")
        .arg("-d")
        .status()?;
    Ok(())
}

fn start_arch_nodes() -> Result<()> {
    println!("Starting Arch Network nodes...");
    Command::new("docker-compose")
        .arg("-f")
        .arg("path/to/arch-docker-compose.yml")
        .arg("up")
        .arg("-d")
        .status()?;
    Ok(())
}

fn load_config() -> Result<Config> {
    let config_path = "config.toml";

    let mut builder = Config::builder();

    // Check if the config file exists
    if Path::new(config_path).exists() {
        builder = builder.add_source(File::with_name(config_path));
        println!("Loading configuration from {}", config_path);
    } else {
        println!("Warning: {} not found. Using default configuration.", config_path);
        // You might want to create a default config here
    }

    // Add environment variables as a source (this will override file settings)
    builder = builder.add_source(Environment::default());

    // Build the configuration
    let config = builder.build().context("Failed to build configuration")?;

    Ok(config)
}

fn check_file_exists(file_path: &str) -> Result<()> {
    if !Path::new(file_path).exists() {
        Err(anyhow!("File not found: {}", file_path))
    } else {
        Ok(())
    }
}

fn set_env_vars(config: &Config) -> Result<()> {
    let vars = [
        ("BITCOIN_RPC_PORT", "bitcoin.rpc_port"),
        ("BITCOIN_RPC_USER", "bitcoin.rpc_user"),
        ("BITCOIN_RPC_PASSWORD", "bitcoin.rpc_password"),
        ("ELECTRS_REST_API_PORT", "electrs.rest_api_port"),
        ("ELECTRS_ELECTRUM_PORT", "electrs.electrum_port"),
        ("BTC_RPC_EXPLORER_PORT", "btc_rpc_explorer.port"),
        ("ORD_PORT", "ord.port"),
        ("NETWORK_MODE", "arch.network_mode"),
        ("RUST_LOG", "arch.rust_log"),
        ("RUST_BACKTRACE", "arch.rust_backtrace"),
        ("BOOTNODE_IP", "arch.bootnode_ip"),
        ("BOOTNODE_P2P_PORT", "arch.bootnode_p2p_port"),
        ("BOOTNODE_PEERID", "arch.bootnode_peerid"),
        ("LEADER_P2P_PORT", "arch.leader_p2p_port"),
        ("LEADER_RPC_PORT", "arch.leader_rpc_port"),
        ("LEADER_PEERID", "arch.leader_peerid"),
        ("VALIDATOR1_P2P_PORT", "arch.validator1_p2p_port"),
        ("VALIDATOR1_RPC_PORT", "arch.validator1_rpc_port"),
        ("VALIDATOR2_P2P_PORT", "arch.validator2_p2p_port"),
        ("VALIDATOR2_RPC_PORT", "arch.validator2_rpc_port"),
        ("BITCOIN_RPC_ENDPOINT", "arch.bitcoin_rpc_endpoint"),
        ("BITCOIN_RPC_WALLET", "arch.bitcoin_rpc_wallet"),
    ];

    for (env_var, config_key) in vars.iter() {
        let value = config
            .get_string(config_key)
            .with_context(|| format!("Failed to get {} from config", config_key))?;
        env::set_var(env_var, value);
    }

    Ok(())
}

fn start_docker_service(
    service_name: &str,
    container_name: &str,
    compose_file: &str
) -> Result<()> {
    let is_running = check_docker_status(container_name).with_context(||
        format!("Failed to check status of {}", service_name)
    )?;

    if !is_running {
        docker_manager
            ::start_docker_compose(compose_file)
            .with_context(|| format!("Failed to start {}", service_name))?;
        println!("{} started.", service_name);
    } else {
        println!("{} already running.", service_name);
    }

    Ok(())
}

fn check_docker_status(container_name: &str) -> Result<bool> {
    let output = Command::new("docker")
        .arg("ps")
        .arg("--format")
        .arg("{{.Names}}")
        .output()
        .context("Failed to execute docker ps command")?;

    let running = String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line == container_name);

    Ok(running)
}

fn create_docker_network(network_name: &str) -> Result<()> {
    let output = Command::new("docker")
        .args(&["network", "create", "--driver", "bridge", network_name])
        .output()
        .context("Failed to execute docker network create command")?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        if error_message.contains("already exists") {
            println!("Network {} already exists, using existing network.", network_name);
        } else {
            return Err(anyhow::anyhow!("Failed to create network: {}", error_message));
        }
    } else {
        println!("Created Docker network: {}", network_name);
    }

    Ok(())
}
fn remove_orphaned_containers(bitcoin_compose_file: &str, arch_compose_file: &str) -> Result<()> {
    println!("Removing orphaned containers...");

    // Remove orphaned containers for Bitcoin setup
    let output = Command::new("docker-compose")
        .args(&["-f", bitcoin_compose_file, "down", "--remove-orphans"])
        .output()
        .context("Failed to remove orphaned containers for Bitcoin setup")?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        println!("Warning: Failed to remove orphaned containers for Bitcoin setup: {}", error_message);
    }

    // Remove orphaned containers for Arch Network setup
    let output = Command::new("docker-compose")
        .args(&["-f", arch_compose_file, "down", "--remove-orphans"])
        .output()
        .context("Failed to remove orphaned containers for Arch Network setup")?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        println!("Warning: Failed to remove orphaned containers for Arch Network setup: {}", error_message);
    }

    println!("Orphaned containers removed");
    Ok(())
}

fn build_program(args: &DeployArgs) -> Result<()> {
    if let Some(path) = &args.directory {
        if !std::path::Path::new(path).exists() {
            return Err(anyhow::anyhow!("Specified directory does not exist: {}", path));
        }
        std::process::Command
            ::new("cargo")
            .args(&["build-sbf", "--manifest-path", &format!("{}/Cargo.toml", path)])
            .status()
            .context("Failed to build program")?;
    } else {
        println!("  {} Building program...", "→".bold().blue());
        std::process::Command
            ::new("cargo")
            .args(&["build-sbf", "--manifest-path", "src/app/program/Cargo.toml"])
            .status()
            .context("Failed to build program")?;
    }
    Ok(())
}
fn get_program_key_path(args: &DeployArgs, config: &Config) -> Result<String> {
    Ok(
        args.program_key
            .clone()
            .unwrap_or_else(|| {
                config
                    .get_string("program.key_path")
                    .unwrap_or_else(|_| PROGRAM_FILE_PATH.to_string())
            })
    )
}

fn setup_bitcoin_rpc_client(config: &Config) -> Result<Client> {
    let endpoint = config
        .get_string("bitcoin.rpc_endpoint")
        .or_else(|_| {
            let host = config
                .get_string("bitcoin.rpc_host")
                .unwrap_or_else(|_| "localhost".to_string());
            let port = config
                .get_string("bitcoin.rpc_port")
                .unwrap_or_else(|_| "18443".to_string());
            Ok::<String, anyhow::Error>(format!("http://{}:{}", host, port))
        })
        .context("Failed to get Bitcoin RPC endpoint")?;

    let username = config
        .get_string("bitcoin.rpc_user")
        .context("Failed to get Bitcoin RPC username")?;
    let password = config
        .get_string("bitcoin.rpc_password")
        .context("Failed to get Bitcoin RPC password")?;
    let wallet_name = config
        .get_string("bitcoin.rpc_wallet")
        .unwrap_or_else(|_| "default".to_string());

    let client = Client::new(&endpoint, Auth::UserPass(username, password)).context(
        "Failed to create RPC client"
    )?;

    // Try to load the wallet
    match client.load_wallet(&wallet_name) {
        Ok(_) => println!("Wallet '{}' loaded successfully.", wallet_name),
        Err(e) => {
            // If the wallet doesn't exist, create it
            if e.to_string().contains("Wallet file verification failed") {
                println!("Wallet '{}' not found. Creating new wallet...", wallet_name);
                client.create_wallet(&wallet_name, None, None, None, None)?;
                println!("Wallet '{}' created successfully.", wallet_name);
            } else {
                return Err(e.into());
            }
        }
    }

    Ok(client)
}
async fn handle_fund_transfer(
    rpc: &Client,
    account_address: &str,
    config: &Config
) -> Result<Option<bitcoincore_rpc::json::GetTransactionResult>> {
    let network = config.get_string("bitcoin.network").unwrap_or_else(|_| "regtest".to_string());
    let bitcoin_network = Network::from_str(&network).context(
        "Invalid Bitcoin network specified in config"
    )?;

    let address = Address::from_str(account_address).context("Invalid account address")?;
    let checked_address = address
        .require_network(bitcoin_network)
        .context("Account address does not match the configured Bitcoin network")?;

    if bitcoin_network == Network::Regtest {
        // Ensure the wallet has funds
        let balance = rpc.get_balance(None, None)?;
        if balance == Amount::ZERO {
            println!("Generating initial blocks to receive mining rewards...");
            let new_address = rpc.get_new_address(None, None)?;
            let checked_address = new_address.require_network(bitcoin_network)?;
            rpc.generate_to_address(101, &checked_address)?;
            println!("Initial blocks generated. Waiting for balance to be available...");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let tx = rpc.send_to_address(
            &checked_address,
            Amount::from_sat(3000),
            None,
            None,
            None,
            None,
            None,
            None
        )?;
        println!("Transaction sent: {}", tx);
        // Generate a block to confirm the transaction
        let new_address = rpc.get_new_address(None, None)?;
        let checked_new_address = new_address.require_network(bitcoin_network)?;
        rpc.generate_to_address(1, &checked_new_address)?;

        // Wait for transaction confirmation
        loop {
            match rpc.get_transaction(&tx, None) {
                Ok(info) if info.info.confirmations > 0 => {
                    println!(
                        "Transaction confirmed with {} confirmations",
                        info.info.confirmations
                    );
                    return Ok(Some(info));
                }
                Ok(_) => println!("Waiting for confirmation..."),
                Err(e) => println!("Error checking transaction: {}", e),
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    } else {
        println!("{}", "Please deposit funds to continue:".bold());
        println!("  {} Deposit address: {}", "→".bold().blue(), account_address.yellow());
        println!("  {} Minimum required: {} satoshis", "ℹ".bold().blue(), "3000".yellow());
        println!("  {} Waiting for funds...", "⏳".bold().blue());

        // TODO: Implement balance checking for non-REGTEST networks
        Ok(None)
    }
}
async fn deploy_program_with_tx_info(
    program_keypair: &bitcoin::secp256k1::Keypair,
    program_pubkey: &arch_program::pubkey::Pubkey,
    tx_info: Option<bitcoincore_rpc::json::GetTransactionResult>
) -> Result<()> {
    if let Some(info) = tx_info {
        deploy_program(program_keypair, program_pubkey, &info.info.txid.to_string(), 0).await;
        Ok(())
    } else {
        println!("Warning: No transaction info available for deployment");
        // You might want to implement an alternative deployment method for non-REGTEST networks
        Ok(())
    }
}
