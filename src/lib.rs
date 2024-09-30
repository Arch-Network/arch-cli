use anyhow::anyhow;
use anyhow::{Context, Result};
use arch_program::account::AccountMeta;
use arch_program::instruction::Instruction;
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction::SystemInstruction;
use bitcoin::Address;
use bitcoin::Amount;
use bitcoin::Network;
use bitcoincore_rpc::jsonrpc::serde_json;
use bitcoincore_rpc::{Client, RpcApi};
use clap::{Args, Parser, Subcommand};
use colored::*;
use common::constants::*;
use common::helper::*;
use config::{Config, Environment, File};
use rand::rngs::OsRng;
use secp256k1::Keypair;
use secp256k1::{Secp256k1, SecretKey};
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as ShellCommand;
use std::process::Command;
use std::str::FromStr;
use std::time::Duration;

use common::wallet_manager::*;

#[derive(Deserialize)]
pub struct ServiceConfig {
    #[allow(dead_code)]
    docker_compose_file: String,
    services: Vec<String>,
}

#[derive(Parser)]
#[clap(
    name = "arch-cli",
    about = "Arch Network CLI - A tool for managing Arch Network applications",
    long_about = None,
    version,
    after_help = "Tip: Use 'arch-cli help <command>' for more information about a specific command."
)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[clap(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new Arch Network app
    #[clap(
        long_about = "Creates the project structure and boilerplate files for a new Arch Network application."
    )]
    Init,

    /// Manage the development server
    #[clap(subcommand)]
    Server(ServerCommands),

    /// Deploy your Arch Network app
    #[clap(
        long_about = "Builds and deploys your Arch Network application to the specified network."
    )]
    Deploy(DeployArgs),

    /// Manage the project
    #[clap(subcommand)]
    Project(ProjectCommands),

    /// Manage the Distributed Key Generation (DKG) process
    #[clap(subcommand)]
    Dkg(DkgCommands),

    /// Manage Bitcoin operations
    #[clap(subcommand)]
    Bitcoin(BitcoinCommands),

    /// Manage the frontend application
    #[clap(subcommand)]
    Demo(DemoCommands),

    /// Manage accounts
    #[clap(subcommand)]
    Account(AccountCommands),

    /// Manage configuration
    #[clap(subcommand)]
    Config(ConfigCommands),

    /// Alias for 'server start'
    #[clap(alias = "up", hide = true)]
    Start,

    /// Alias for 'server stop'
    #[clap(alias = "down", hide = true)]
    Stop,

    /// Manage the indexer
    #[clap(subcommand)]
    Indexer(IndexerCommands),

    /// Manage the validator
    #[clap(subcommand)]
    Validator(ValidatorCommands),
}

#[derive(Subcommand)]
pub enum ServerCommands {
    /// Start the development server
    #[clap(
        long_about = "Starts the development environment, including Bitcoin regtest network and Arch Network nodes."
    )]
    Start,

    /// Stop the development server
    #[clap(
        long_about = "Stops all related Docker containers and services for the development environment."
    )]
    Stop,

    /// Check the status of the development server
    #[clap(
        long_about = "Displays the current status of all services in the development environment."
    )]
    Status,

    /// View logs for development server components
    #[clap(long_about = "Displays logs for specified services in the development environment.")]
    Logs {
        /// Specify which service to show logs for (e.g., 'bitcoin', 'arch')
        #[clap(default_value = "all")]
        service: String,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Clean the project
    #[clap(long_about = "Removes the src/app directory, cleaning the project structure.")]
    Clean,
}

#[derive(Subcommand)]
pub enum IndexerCommands {
    /// Start the indexer
    #[clap(long_about = "Starts the arch-indexer using Docker Compose.")]
    Start,

    /// Stop the indexer
    #[clap(long_about = "Stops the arch-indexer using Docker Compose.")]
    Stop,
}

#[derive(Subcommand)]
pub enum ValidatorCommands {
    /// Start the validator
    #[clap(long_about = "Starts a local validator with specified network settings.")]
    Start(ValidatorStartArgs),

    /// Stop the validator
    #[clap(long_about = "Stops the local validator.")]
    Stop,
}

#[derive(Subcommand)]
pub enum DkgCommands {
    /// Start the Distributed Key Generation (DKG) process
    #[clap(long_about = "Initiates the Distributed Key Generation process on the Arch Network.")]
    Start,
}

#[derive(Subcommand)]
pub enum BitcoinCommands {
    /// Send coins to an address on Regtest
    #[clap(long_about = "Sends coins to a specified address on the Bitcoin Regtest network.")]
    SendCoins(SendCoinsArgs),
}

#[derive(Subcommand)]
pub enum DemoCommands {
    /// Start the demo application (frontend and backend)
    #[clap(
        long_about = "Starts the demo application, including both frontend and backend services."
    )]
    Start,

    /// Stop the demo application (frontend and backend)
    #[clap(
        long_about = "Stops the demo application, including both frontend and backend services."
    )]
    Stop,
}

#[derive(Subcommand)]
pub enum AccountCommands {
    /// Create an account for the dApp
    #[clap(
        long_about = "Creates an account for the dApp, prompts for funding, and transfers ownership to the program"
    )]
    Create(CreateAccountArgs),

    /// List all accounts
    #[clap(long_about = "Lists all accounts stored in the accounts.json file")]
    List,

    /// Delete an account
    #[clap(long_about = "Deletes an account from the accounts.json file")]
    Delete(DeleteAccountArgs),
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// View current configuration
    View,
    /// Edit configuration
    Edit,
    /// Reset configuration to default
    Reset,
}

#[derive(Args)]
pub struct CreateAccountArgs {
    /// Program ID to transfer ownership to
    #[clap(long, help = "Specifies the program ID to transfer ownership to")]
    program_id: Option<String>,
    /// Custom name for the account
    #[clap(long, help = "Specifies a custom name for the account")]
    name: String,
}

#[derive(Args)]
pub struct DeleteAccountArgs {
    /// Account ID or name to delete
    #[clap(help = "Specifies the account ID or name to delete")]
    identifier: String,
}

#[derive(Args)]
pub struct DeployArgs {
    /// Directory of your program
    #[clap(
        long,
        help = "Specifies the directory containing your Arch Network program"
    )]
    directory: Option<String>,

    /// Path to the program key file
    #[clap(
        long,
        help = "Specifies the path to the program's key file for deployment"
    )]
    program_key: Option<String>,
}

#[derive(Args)]
pub struct SendCoinsArgs {
    /// Address to send coins to
    #[clap(long, help = "Specifies the address to send coins to")]
    address: String,
    /// Amount to send
    #[clap(long, help = "Specifies the amount of coins to send")]
    amount: u64,
}

#[derive(Args)]
pub struct ValidatorStartArgs {
    /// Network to use (testnet or mainnet)
    #[clap(long, default_value = "testnet", help = "Specifies the network to use: testnet or mainnet")]
    network: String,
}

pub async fn init() -> Result<()> {
    println!("{}", "Initializing new Arch Network app...".bold().green());

    // Check dependencies
    check_dependencies()?;

    // Get the configuration file path
    let config_path = get_config_path()?;

    // Create the arch-data directory
    let config_dir = config_path.parent().unwrap();
    let arch_data_dir = config_dir.join("arch-data");
    fs::create_dir_all(&arch_data_dir)?;
    println!(
        "  {} Created arch-data directory at {:?}",
        "✓".bold().green(),
        arch_data_dir
    );

    // Copy config.default.toml to the arch_data_dir
    let default_config_path = Path::new("config.default.toml");
    if default_config_path.exists() {
        let dest_path = config_dir.join("config.toml");
        fs::copy(default_config_path, &dest_path)
            .with_context(|| format!("Failed to copy default config to {:?}", dest_path))?;
        println!(
            "  {} Copied default configuration to {:?}",
            "✓".bold().green(),
            dest_path
        );
    } else {
        println!(
            "  {} Warning: config.default.toml not found",
            "⚠".bold().yellow()
        );
    }

    // Navigate to the program folder and run `cargo build-sbf`
    println!("{}", "Building Arch Network program...".bold().blue());
    ShellCommand::new("cargo")
        .current_dir("program")
        .arg("build-sbf")
        .output()
        .expect("Failed to build Arch Network program");

    // // Create project structure
    // println!("{}", "Creating project structure...".bold().blue());
    // let dirs = ["src/app/backend", "src/app/keys"];
    // for dir in dirs.iter() {
    //     fs::create_dir_all(dir)
    //         .with_context(|| format!("Failed to create directory: {}", dir.yellow()))?;
    // }

    // // Create boilerplate files
    // println!("{}", "Creating boilerplate files...".bold().blue());
    // let files = [
    //     ("src/app/backend/index.ts", include_str!("../templates/backend_index.ts")),
    //     ("src/app/backend/package.json", include_str!("../templates/backend_package.json")),
    // ];

    // for (file_path, content) in files.iter() {
    //     if !Path::new(file_path).exists() {
    //         fs::write(file_path, content)
    //             .with_context(|| format!("Failed to write file: {}", file_path))?;
    //     } else {
    //         println!("  {} File already exists, skipping: {}", "ℹ".bold().blue(), file_path);
    //     }
    // }

    // Check if program and frontend directories exist
    let program_dir = Path::new("src/app/program");
    // let frontend_dir = Path::new("src/app/frontend");

    if !program_dir.exists() {
        println!("  {} Creating default program directory", "→".bold().blue());
        fs::create_dir_all(program_dir.join("src"))?;
        fs::write(
            program_dir.join("src/lib.rs"),
            include_str!("../templates/program_lib.rs"),
        )?;
        fs::write(
            program_dir.join("Cargo.toml"),
            include_str!("../templates/program_cargo.toml"),
        )?;

        println!("  {} Default program files created", "✓".bold().green());
    } else {
        println!(
            "  {} Existing program directory found, preserving it",
            "ℹ".bold().blue()
        );
    }

    // if !frontend_dir.exists() {
    //     println!("  {} Creating default frontend directory", "→".bold().blue());
    //     fs::create_dir_all(frontend_dir)?;
    //     fs::write(
    //         frontend_dir.join("index.html"),
    //         include_str!("../templates/frontend_index.html")
    //     )?;
    //     fs::write(
    //         frontend_dir.join("index.js"),
    //         include_str!("../templates/frontend_index.js")
    //     )?;
    //     fs::write(
    //         frontend_dir.join("package.json"),
    //         include_str!("../templates/frontend_package.json")
    //     )?;
    // } else {
    //     println!("  {} Existing frontend directory found, preserving it", "ℹ".bold().blue());
    // }

    println!(
        "  {} New Arch Network app initialized successfully!",
        "✓".bold().green()
    );
    Ok(())
}

fn get_config_path() -> Result<PathBuf> {
    let config_path = env::var("ARCH_CLI_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut default_path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
            default_path.push("arch-cli");
            default_path.push("config.toml");
            default_path
        });
    Ok(config_path)
}

fn get_docker_compose_command() -> (&'static str, &'static [&'static str]) {
    if Command::new("docker-compose")
        .arg("--version")
        .output()
        .is_ok()
    {
        ("docker-compose", &[])
    } else {
        ("docker", &["compose"])
    }
}

fn check_dependencies() -> Result<()> {
    println!("{}", "Checking required dependencies...".bold().blue());

    static DEPENDENCIES: &[(&str, &[&[&str]], &str)] = &[
        ("docker", &[&["docker", "--version"]], "Docker is not installed. Please install Docker."),
        (
            "docker-compose",
            &[
                &["docker-compose", "--version"],
                &["docker", "compose", "--version"],
            ],
            "Neither docker-compose nor docker compose is available. Please install Docker Compose."
        ),
        ("node", &[&["node", "--version"]], "Node.js is not installed or version is below 19. Please install Node.js version 19 or higher."),
        ("solana", &[&["solana", "--version"]], "Solana CLI is not installed. Please install Solana CLI."),
        ("cargo", &[&["cargo", "--version"]], "Rust and Cargo are not installed. Please install Rust and Cargo."),
    ];

    for (name, commands, error_message) in DEPENDENCIES.iter() {
        print!("  {} Checking {}...", "→".bold().blue(), name);
        io::stdout().flush()?;

        let mut success = false;
        let mut version = String::new();

        for command in *commands {
            match Command::new(command[0]).args(&command[1..]).output() {
                Ok(output) if output.status.success() => {
                    version = String::from_utf8_lossy(&output.stdout).to_string();
                    success = true;
                    break;
                }
                _ => continue,
            }
        }

        if success {
            println!(" {}", "✓".bold().green());
            println!("    Detected version: {}", version.trim());

            // Additional check for Node.js version
            if *name == "node" {
                let version_str = version.split('v').nth(1).unwrap_or("").trim();
                let major_version = version_str
                    .split('.')
                    .next()
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0);
                if major_version < 19 {
                    println!(" {}", "✗".bold().red());
                    println!("{}", error_message);
                    return Err(anyhow::Error::msg(error_message));
                }
            }
        } else {
            println!(" {}", "✗".bold().red());
            println!("{}", error_message);
            return Err(anyhow::Error::msg(error_message));
        }
    }

    println!(
        "{}",
        "All required dependencies are installed.".bold().green()
    );
    Ok(())
}

fn _start_or_create_services(service_name: &str, service_config: &ServiceConfig) -> Result<()> {
    println!(
        "  {} Starting {}...",
        "→".bold().blue(),
        service_name.yellow()
    );

    let mut all_containers_exist = true;
    let mut all_containers_running = true;

    for container in &service_config.services {
        let ps_output = Command::new("docker-compose")
            .args([
                "-f",
                &service_config.docker_compose_file,
                "ps",
                "-q",
                container,
            ])
            .output()
            .context(format!(
                "Failed to check existing container for {}",
                container
            ))?;

        if ps_output.stdout.is_empty() {
            all_containers_exist = false;
            all_containers_running = false;
            break;
        }

        let status_output = Command::new("docker")
            .args([
                "inspect",
                "-f",
                "{{.State.Running}}",
                String::from_utf8_lossy(&ps_output.stdout).trim(),
            ])
            .output()
            .context(format!("Failed to check status of container {}", container))?;

        if String::from_utf8_lossy(&status_output.stdout).trim() != "true" {
            all_containers_running = false;
        }
    }

    if all_containers_exist {
        if all_containers_running {
            println!(
                "  {} All {} containers are already running.",
                "✓".bold().green(),
                service_name.yellow()
            );
        } else {
            println!(
                "  {} Existing {} containers found. Starting them...",
                "→".bold().blue(),
                service_name.yellow()
            );
            let start_output = Command::new("docker-compose")
                .args(["-f", &service_config.docker_compose_file, "start"])
                .output()
                .context(format!(
                    "Failed to start existing {} containers",
                    service_name
                ))?;

            if !start_output.status.success() {
                let error_message = String::from_utf8_lossy(&start_output.stderr);
                println!(
                    "  {} Warning: Failed to start some {} containers: {}",
                    "⚠".bold().yellow(),
                    service_name.yellow(),
                    error_message.red()
                );
                return Err(anyhow!("Failed to start some {} containers", service_name));
            } else {
                println!(
                    "  {} {} containers started successfully.",
                    "✓".bold().green(),
                    service_name.yellow()
                );
            }
        }
    } else {
        println!(
            "  {} Some or all {} containers are missing. Creating and starting new ones...",
            "ℹ".bold().blue(),
            service_name.yellow()
        );
        let up_output = Command::new("docker-compose")
            .args([
                "--progress",
                "auto",
                "-f",
                &service_config.docker_compose_file,
                "up",
                "--build",
                "-d",
            ])
            .envs(std::env::vars())
            .output()
            .context(format!(
                "Failed to create and start {} containers",
                service_name
            ))?;

        if !up_output.status.success() {
            let error_message = String::from_utf8_lossy(&up_output.stderr);
            if error_message.contains("invalid reference format") {
                println!(
                    "  {} Error: Invalid reference format in Docker image name. Please check your Docker image names and try again.",
                    "✗".bold().red()
                );
            } else if let Some(variable) = error_message.split("variable is not set: ").nth(1) {
                println!(
                    "  {} Error: Environment variable '{}' is not set. Please ensure all required environment variables are set and try again.",
                    "✗".bold().red(),
                    variable.trim()
                );
            } else {
                println!(
                    "  {} Warning: Failed to create and start {} containers: {}",
                    "⚠".bold().yellow(),
                    service_name.yellow(),
                    error_message.red()
                );
            }
            return Err(anyhow!(
                "Failed to create and start {} containers",
                service_name
            ));
        } else {
            println!(
                "  {} {} containers created and started successfully.",
                "✓".bold().green(),
                service_name.yellow()
            );
        }
    }

    Ok(())
}

pub async fn server_start(config: &Config) -> Result<()> {
    println!("{}", "Starting the development server...".bold().green());

    let arch_data_dir = get_arch_data_dir(config)?;

    // Set the ARCH_DATA_DIR environment variable
    env::set_var("ARCH_DATA_DIR", arch_data_dir.to_str().unwrap());

    // Set other required environment variables
    set_env_vars(config)?;

    // Start Bitcoin services
    start_docker_service(
        "Bitcoin",
        "bitcoin",
        &config.get_string("bitcoin.docker_compose_file")?,
    )?;

    // Start Arch Network services
    let arch_compose_file = config.get_string("arch.docker_compose_file")?;
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();

    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(["-f", &arch_compose_file, "up", "-d"])
        .env("ARCH_DATA_DIR", arch_data_dir.to_str().unwrap())
        .status()?;

    // Start the DKG process
    start_dkg(config).await?;

    println!(
        "  {} Development server started successfully.",
        "✓".bold().green()
    );

    Ok(())
}

pub async fn deploy(args: &DeployArgs, config: &Config) -> Result<()> {
    println!("{}", "Deploying your Arch Network app...".bold().green());

    // Build the program
    build_program(args)?;

    // Ensure the keys directory exists and load/generate the program keypair
    let (program_keypair, program_pubkey) = prepare_program_keys()?;

    // Display the program public key
    display_program_id(&program_pubkey);

    // Set up Bitcoin RPC client and handle funding
    let wallet_manager = WalletManager::new(config)?;
    ensure_wallet_balance(&wallet_manager.client).await?;

    // Get account address and fund it
    let account_address = get_account_address_async(program_pubkey).await?;
    let tx_info = fund_address(&wallet_manager.client, &account_address, config).await?;

    // Deploy the program
    deploy_program_with_tx_info(&program_keypair, &program_pubkey, tx_info).await?;

    wallet_manager.close_wallet()?;

    println!(
        "{}",
        "Your app has been deployed successfully!".bold().green()
    );
    display_program_id(&program_pubkey);

    Ok(())
}

pub async fn server_stop() -> Result<()> {
    println!("{}", "Stopping development server...".bold().yellow());

    stop_all_related_containers()?;

    println!(
        "{}",
        "Development server stopped successfully!".bold().green()
    );
    println!(
        "{}",
        "You can restart the server later using the 'server start' command.".italic()
    );
    Ok(())
}

pub async fn send_coins(args: &SendCoinsArgs, config: &Config) -> Result<()> {
    // Initialize the WalletManager
    let wallet_manager = WalletManager::new(config)?;

    // Check wallet balance before sending
    let balance = wallet_manager.client.get_balance(None, None)?;

    if balance < Amount::from_sat(args.amount) {
        return Err(anyhow!(
            "Insufficient balance. Available: {}, Required: {}",
            balance.to_string().yellow(),
            Amount::from_sat(args.amount).to_string().yellow()
        ));
    }

    // Parse the destination address
    let address = Address::from_str(&args.address)?;
    println!(
        "  {} Sending {} satoshis to address: {}",
        "ℹ".bold().blue(),
        args.amount.to_string().yellow(),
        args.address.yellow()
    );

    let address_networked = address.require_network(Network::Regtest)?;

    // Send the coins
    let txid = wallet_manager.client.send_to_address(
        &address_networked,
        Amount::from_sat(args.amount),
        None,
        None,
        None,
        None,
        None,
        None,
    )?;

    // Generate 1 block to confirm the transaction
    wallet_manager
        .client
        .generate_to_address(1, &address_networked)?;

    // Print success message
    println!(
        "{} Coins sent successfully! Transaction ID: {}",
        "✓".bold().green(),
        txid.to_string().yellow()
    );

    // Close the wallet if needed
    wallet_manager.close_wallet()?;

    Ok(())
}

fn stop_all_related_containers() -> Result<()> {
    let container_prefixes = vec!["arch-cli", "bitcoin", "electrs", "btc-rpc-explorer"];

    for prefix in container_prefixes {
        println!(
            "  {} Stopping {} containers...",
            "→".bold().blue(),
            prefix.yellow()
        );

        // List all running containers with the given prefix
        let output = Command::new("docker")
            .args(["ps", "-q", "--filter", &format!("name={}", prefix)])
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
                println!(
                    "  {} Warning: Failed to stop some {} containers: {}",
                    "⚠".bold().yellow(),
                    prefix.yellow(),
                    error_message.red()
                );
            } else {
                println!(
                    "  {} {} containers stopped successfully.",
                    "✓".bold().green(),
                    prefix.yellow()
                );
            }
        } else {
            println!(
                "  {} No running {} containers found to stop.",
                "ℹ".bold().blue(),
                prefix.yellow()
            );
        }
    }

    Ok(())
}

pub async fn server_status(config: &Config) -> Result<()> {
    println!("{}", "Checking development server status...".bold().blue());

    let network_type = config
        .get_string("network.type")
        .context("Failed to get network type from configuration")?;

    if network_type == "development" {
        let bitcoin_config: ServiceConfig = config
            .get("bitcoin")
            .context("Failed to get Bitcoin configuration")?;
        check_service_status("Bitcoin regtest network", &bitcoin_config)?;

        let arch_config: ServiceConfig = config
            .get("arch")
            .context("Failed to get Arch Network configuration")?;
        check_service_status("Arch Network nodes", &arch_config)?;
    } else {
        println!(
            "  {} Using existing network configuration for: {}",
            "ℹ".bold().blue(),
            network_type.yellow()
        );
    }

    Ok(())
}

fn fetch_service_logs(service_name: &str, service_config: &ServiceConfig) -> Result<()> {
    println!(
        "  {} Fetching logs for {}...",
        "→".bold().blue(),
        service_name.yellow()
    );

    for container in &service_config.services {
        println!("    Logs for {}:", container.bold());
        let log_output = Command::new("docker")
            .args(["logs", "--tail", "50", container])
            .output()
            .context(format!("Failed to fetch logs for container {}", container))?;

        println!("{}", String::from_utf8_lossy(&log_output.stdout));
    }

    Ok(())
}

fn check_service_status(service_name: &str, service_config: &ServiceConfig) -> Result<()> {
    println!(
        "  {} Checking {} status...",
        "→".bold().blue(),
        service_name.yellow()
    );

    for container in &service_config.services {
        let status_output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                &format!("name={}", container),
                "--format",
                "{{.Status}}",
            ])
            .output()
            .context(format!("Failed to check status of container {}", container))?;

        let status = String::from_utf8_lossy(&status_output.stdout)
            .trim()
            .to_string();

        if status.starts_with("Up") {
            println!("    {} {} is running", "✓".bold().green(), container);
        } else if status.is_empty() {
            println!("    {} {} is not created", "✗".bold().red(), container);
        } else {
            println!(
                "    {} {} is not running (status: {})",
                "✗".bold().red(),
                container,
                status
            );
        }
    }

    Ok(())
}

pub async fn server_logs(service: &str, config: &Config) -> Result<()> {
    println!(
        "{}",
        format!("Fetching logs for {}...", service).bold().blue()
    );

    let network_type = config
        .get_string("network.type")
        .context("Failed to get network type from configuration")?;

    if network_type == "development" {
        if service == "all" || service == "bitcoin" {
            let bitcoin_config: ServiceConfig = config
                .get("bitcoin")
                .context("Failed to get Bitcoin configuration")?;
            fetch_service_logs("Bitcoin regtest network", &bitcoin_config)?;
        }

        if service == "all" || service == "arch" {
            let arch_config: ServiceConfig = config
                .get("arch")
                .context("Failed to get Arch Network configuration")?;
            fetch_service_logs("Arch Network nodes", &arch_config)?;
        }
    } else {
        println!(
            "  {} Logs are not available for non-development networks",
            "ℹ".bold().blue()
        );
    }

    Ok(())
}

pub fn start_existing_containers(compose_file: &str) -> Result<()> {
    let output = Command::new("docker-compose")
        .args(["-f", compose_file, "ps", "-q"])
        .output()
        .context("Failed to list existing containers")?;

    if !output.stdout.is_empty() {
        println!(
            "  {} Found existing containers. Starting them...",
            "→".bold().blue()
        );
        let start_output = Command::new("docker-compose")
            .args(["-f", compose_file, "start"])
            .output()
            .context("Failed to start existing containers")?;

        if !start_output.status.success() {
            let error_message = String::from_utf8_lossy(&start_output.stderr);
            println!(
                "  {} Warning: Failed to start some containers: {}",
                "⚠".bold().yellow(),
                error_message.red()
            );
        } else {
            println!(
                "  {} Existing containers started successfully.",
                "✓".bold().green()
            );
        }
    } else {
        println!(
            "  {} No existing containers found. Creating new ones...",
            "ℹ".bold().blue()
        );
        // Proceed with your existing logic to create new containers
    }

    Ok(())
}

pub fn remove_docker_networks() -> Result<()> {
    let networks = vec!["arch-network", "internal"];

    for network in networks {
        println!(
            "  {} Removing Docker network: {}",
            "→".bold().blue(),
            network.yellow()
        );

        let output = Command::new("docker")
            .args(["network", "rm", network])
            .output()
            .context(format!("Failed to remove Docker network: {}", network))?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            if error_message.contains("not found") {
                println!(
                    "  {} Network {} not found. Skipping.",
                    "ℹ".bold().blue(),
                    network.yellow()
                );
            } else {
                println!(
                    "  {} Warning: Failed to remove network {}: {}",
                    "⚠".bold().yellow(),
                    network.yellow(),
                    error_message.red()
                );
            }
        } else {
            println!(
                "  {} Network {} removed successfully.",
                "✓".bold().green(),
                network.yellow()
            );
        }
    }

    Ok(())
}

pub fn stop_docker_services(compose_file: &str, service_name: &str) -> Result<()> {
    println!(
        "  {} Stopping {} services...",
        "→".bold().blue(),
        service_name.yellow()
    );
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();

    let output = Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(["-f", compose_file, "down"])
        .output()?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        println!(
            "  {} Warning: Failed to stop {} services: {}",
            "⚠".bold().yellow(),
            service_name.yellow(),
            error_message.red()
        );
    } else {
        println!(
            "  {} {} services stopped successfully.",
            "✓".bold().green(),
            service_name.yellow()
        );
    }

    Ok(())
}

pub async fn clean() -> Result<()> {
    println!("{}", "Cleaning up the project...".bold().yellow());
    let config = load_config()?;
    let arch_data_dir = get_arch_data_dir(&config)?;
    if arch_data_dir.exists() {
        fs::remove_dir_all(&arch_data_dir)?;
        println!("  {} Removed arch-data directory", "✓".bold().green());
    }
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();
    // Stop and remove Docker containers
    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args([
            "-f",
            &config
                .get_string("bitcoin.docker_compose_file")
                .unwrap_or_default(),
            "down",
            "-v",
        ])
        .status()?;
    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args([
            "-f",
            &config
                .get_string("arch.docker_compose_file")
                .unwrap_or_default(),
            "down",
            "-v",
        ])
        .status()?;
    println!("  {} Project cleaned up successfully", "✓".bold().green());
    Ok(())
}

pub fn start_bitcoin_regtest() -> Result<()> {
    println!(
        "  {} Starting Bitcoin regtest network...",
        "→".bold().blue()
    );
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();

    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(["-f", "path/to/bitcoin-docker-compose.yml", "up", "-d"])
        .status()?;

    println!(
        "  {} Bitcoin regtest network started successfully.",
        "✓".bold().green()
    );
    Ok(())
}

pub fn stop_bitcoin_regtest() -> Result<()> {
    println!(
        "  {} Stopping Bitcoin regtest network...",
        "→".bold().blue()
    );
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();

    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(["-f", "path/to/bitcoin-docker-compose.yml", "down"])
        .status()?;

    println!(
        "  {} Bitcoin regtest network stopped successfully.",
        "✓".bold().green()
    );
    Ok(())
}

pub async fn start_dkg(config: &Config) -> Result<()> {
    println!(
        "{}",
        "Starting Distributed Key Generation (DKG) process..."
            .bold()
            .green()
    );

    let leader_rpc = config
        .get_string("arch.leader_rpc_endpoint")
        .expect("Failed to get leader RPC endpoint from config");

    // Create an HTTP client with a timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Prepare the RPC request
    let rpc_request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "start_dkg",
        "params": [],
        "id": 1
    });

    // Check if the leader node is up
    loop {
        match client.get(&leader_rpc).send().await {
            Ok(_) => {
                println!("  {} Leader node is up", "✓".bold().green());
                break;
            }
            Err(e) => {
                println!(
                    "  {} Leader node is not up yet, retrying... ({})",
                    "⚠".bold().yellow(),
                    e
                );
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    // Attempt to start the DKG process
    loop {
        // Send the RPC request
        let response = client
            .post(&leader_rpc)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send RPC request: {:?}", e))?;

        // Check the response
        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .await
                .context("Failed to parse JSON response")?;

            if let Some(error) = result.get("error") {
                let error_message = error["message"].as_str().unwrap_or("Unknown error");
                if error_message == "dkg already occured" {
                    println!("  {} DKG process already occurred", "✓".bold().green());
                    break;
                } else if error_message == "node not ready for dkg" {
                    println!(
                        "  {} Node not ready for DKG, retrying...",
                        "⚠".bold().yellow()
                    );
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                } else {
                    println!(
                        "  {} Failed to start DKG process: {}",
                        "✗".bold().red(),
                        error_message
                    );
                    return Err(anyhow!(error_message.to_string()));
                }
            } else {
                println!("  {} DKG process started successfully", "✓".bold().green());
                println!(
                    "  {} Response: {}",
                    "ℹ".bold().blue(),
                    serde_json::to_string_pretty(&result).unwrap()
                );
            }
        } else {
            let error_message = response
                .text()
                .await
                .context("Failed to get error message")?;
            println!("  {} Failed to start DKG process", "✗".bold().red());
            println!("  {} Error: {}", "ℹ".bold().blue(), error_message);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    // Ensure the DKG process has occurred
    loop {
        let response = client
            .post(&leader_rpc)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send RPC request: {:?}", e))?;

        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .await
                .context("Failed to parse JSON response")?;

            if let Some(error) = result.get("error") {
                let error_message = error["message"].as_str().unwrap_or("Unknown error");
                if error_message == "dkg already occured" {
                    println!("  {} DKG process already occurred", "✓".bold().green());
                    break;
                } else {
                    println!(
                        "  {} Waiting for DKG process to complete...",
                        "⚠".bold().yellow()
                    );
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        } else {
            let error_message = response
                .text()
                .await
                .context("Failed to get error message")?;
            println!("  {} Failed to check DKG process status", "✗".bold().red());
            println!("  {} Error: {}", "ℹ".bold().blue(), error_message);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    Ok(())
}

pub fn start_arch_nodes() -> Result<()> {
    println!("  {} Starting Arch Network nodes...", "→".bold().blue());
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();

    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(["-f", "path/to/arch-docker-compose.yml", "up", "-d"])
        .status()?;

    println!(
        "  {} Arch Network nodes started successfully.",
        "✓".bold().green()
    );
    Ok(())
}

pub fn stop_arch_nodes() -> Result<()> {
    println!("  {} Stopping Arch Network nodes...", "→".bold().blue());
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();

    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(["-f", "path/to/arch-docker-compose.yml", "down"])
        .status()?;

    println!(
        "  {} Arch Network nodes stopped successfully.",
        "✓".bold().green()
    );
    Ok(())
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;
    let config_dir = config_path.parent().unwrap().to_str().unwrap().to_string();

    let mut builder = Config::builder();

    // Check if the config file exists
    if config_path.exists() {
        builder = builder.add_source(File::with_name(config_path.to_str().unwrap()));
        println!(
            "  {} Loading configuration from {}",
            "→".bold().blue(),
            config_path.display().to_string().yellow()
        );
    } else {
        println!(
            "  {} Warning: {} not found. Using default configuration.",
            "⚠".bold().yellow(),
            config_path.display().to_string().yellow()
        );
        // You might want to create a default config here
    }

    builder = builder.add_source(Environment::with_prefix("ARCH_CLI"));

    // Add config_dir to the builder
    builder = builder.set_override("config_dir", config_dir)?;

    let config = builder.build().context("Failed to build configuration")?;

    Ok(config)
}

pub fn get_arch_data_dir(config: &Config) -> Result<PathBuf> {
    let config_dir = config.get_string("config_dir")?;
    Ok(PathBuf::from(config_dir).join("arch-data"))
}

pub fn check_file_exists(file_path: &str) -> Result<()> {
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
        ("BITCOIN_RPC_WALLET", "bitcoin.rpc_wallet"),
        ("ELECTRS_REST_API_PORT", "electrs.rest_api_port"),
        ("ELECTRS_ELECTRUM_PORT", "electrs.electrum_port"),
        ("BTC_RPC_EXPLORER_PORT", "btc_rpc_explorer.port"),
        ("DEMO_FRONTEND_PORT", "demo.frontend_port"),
        ("DEMO_BACKEND_PORT", "demo.backend_port"),
        ("INDEXER_PORT", "indexer.port"),
        ("ORD_PORT", "ord.port"),
        ("NETWORK_MODE", "arch.network_mode"),
        ("RUST_LOG", "arch.rust_log"),
        ("RUST_BACKTRACE", "arch.rust_backtrace"),
        ("BOOTNODE_IP", "arch.bootnode_ip"),
        ("LEADER_P2P_PORT", "arch.leader_p2p_port"),
        ("LEADER_RPC_PORT", "arch.leader_rpc_port"),
        ("VALIDATOR1_P2P_PORT", "arch.validator1_p2p_port"),
        ("VALIDATOR1_RPC_PORT", "arch.validator1_rpc_port"),
        ("VALIDATOR2_P2P_PORT", "arch.validator2_p2p_port"),
        ("VALIDATOR2_RPC_PORT", "arch.validator2_rpc_port"),
        ("BITCOIN_RPC_ENDPOINT", "arch.bitcoin_rpc_endpoint"),
        ("BITCOIN_RPC_WALLET", "arch.bitcoin_rpc_wallet"),
        ("REPLICA_COUNT", "arch.replica_count"),
    ];

    for (env_var, config_key) in vars.iter() {
        let value = config
            .get_string(config_key)
            .with_context(|| format!("Failed to get {} from config", config_key))?;
        env::set_var(env_var, value);
    }

    Ok(())
}

pub fn start_docker_service(
    service_name: &str,
    container_name: &str,
    compose_file: &str,
) -> Result<()> {
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();

    let is_running = check_docker_status(container_name)?;

    if !is_running {
        let output = Command::new(docker_compose_cmd)
            .args(docker_compose_args)
            .args(["-f", compose_file, "up", "-d"])
            .output()?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Failed to start {}: {}",
                service_name,
                error_message
            ));
        }
        println!(
            "  {} {} started.",
            "✓".bold().green(),
            service_name.yellow()
        );
    } else {
        println!(
            "  {} {} already running.",
            "ℹ".bold().blue(),
            service_name.yellow()
        );
    }

    Ok(())
}

pub fn check_docker_status(container_name: &str) -> Result<bool> {
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

fn _create_docker_network(network_name: &str) -> Result<()> {
    let output = Command::new("docker")
        .args(["network", "create", "--driver", "bridge", network_name])
        .output()
        .context("Failed to execute docker network create command")?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        if error_message.contains("already exists") {
            println!(
                "  {} Network {} already exists, using existing network.",
                "ℹ".bold().blue(),
                network_name.yellow()
            );
        } else {
            return Err(anyhow::anyhow!(
                "Failed to create network: {}",
                error_message
            ));
        }
    } else {
        println!(
            "  {} Created Docker network: {}",
            "✓".bold().green(),
            network_name.yellow()
        );
    }

    Ok(())
}

fn get_program_path(args: &DeployArgs) -> PathBuf {
    let mut path = PathBuf::from(
        args.directory
            .clone()
            .unwrap_or_else(|| "src/app/program".to_string()),
    );
    path.push("Cargo.toml");
    path
}

fn build_program(args: &DeployArgs) -> Result<()> {
    println!("  ℹ Building program...");

    let path = get_program_path(args);
    if !std::path::Path::new(&path).exists() {
        return Err(anyhow!("Cargo.toml not found at: {}", path.display()));
    }

    let output = std::process::Command::new("cargo")
        .args(["build-sbf", "--manifest-path", path.to_str().unwrap()])
        .output()
        .context("Failed to execute cargo build-sbf")?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        println!("Build process encountered an error:");
        println!("{}", error_message);
        return Err(anyhow!("Build failed"));
    }

    println!("  ✓ Program built successfully");
    Ok(())
}
fn _get_program_key_path(args: &DeployArgs, config: &Config) -> Result<String> {
    Ok(args.program_key.clone().unwrap_or_else(|| {
        config
            .get_string("program.key_path")
            .unwrap_or_else(|_| PROGRAM_FILE_PATH.to_string())
    }))
}

async fn deploy_program_with_tx_info(
    program_keypair: &bitcoin::secp256k1::Keypair,
    program_pubkey: &arch_program::pubkey::Pubkey,
    tx_info: Option<bitcoincore_rpc::json::GetTransactionResult>,
) -> Result<()> {
    if let Some(info) = tx_info {
        deploy_program(
            program_keypair,
            program_pubkey,
            &info.info.txid.to_string(),
            0,
        )
        .await?;
        println!("  {} Program deployed successfully", "✓".bold().green());
        Ok(())
    } else {
        println!(
            "  {} Warning: No transaction info available for deployment",
            "⚠".bold().yellow()
        );
        // You might want to implement an alternative deployment method for non-REGTEST networks
        Ok(())
    }
}

fn prepare_program_keys() -> Result<(Keypair, Pubkey)> {
    let keys_dir = ensure_keys_dir()?;
    let program_key_path = keys_dir.join("program.json");
    with_secret_key_file(program_key_path.to_str().unwrap())
}

fn display_program_id(program_pubkey: &Pubkey) {
    let program_pubkey_hex = hex::encode(program_pubkey.serialize());
    println!(
        "  {} Program ID: {}",
        "ℹ".bold().blue(),
        program_pubkey_hex.yellow()
    );
}

async fn ensure_wallet_balance(client: &Client) -> Result<()> {
    let balance = client.get_balance(None, None)?;
    if balance == Amount::ZERO {
        println!(
            "  {} Generating initial blocks for mining rewards...",
            "→".blue()
        );
        let new_address = client.get_new_address(None, None)?;
        let checked_address = new_address.require_network(Network::Regtest)?;
        client.generate_to_address(101, &checked_address)?;
        println!("  {} Initial blocks generated", "✓".green());
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}
async fn fund_address(
    rpc: &Client,
    account_address: &str,
    config: &Config,
) -> Result<Option<bitcoincore_rpc::json::GetTransactionResult>> {
    let network = config
        .get_string("bitcoin.network")
        .unwrap_or_else(|_| "regtest".to_string());
    let bitcoin_network =
        Network::from_str(&network).context("Invalid Bitcoin network specified in config")?;

    let address = Address::from_str(account_address).context("Invalid account address")?;
    let checked_address = address
        .require_network(bitcoin_network)
        .context("Account address does not match the configured Bitcoin network")?;

    if bitcoin_network == Network::Regtest {
        // Ensure the wallet has funds
        let balance = rpc.get_balance(None, None)?;
        if balance == Amount::ZERO {
            println!(
                "  {} Generating initial blocks to receive mining rewards...",
                "→".bold().blue()
            );
            let new_address = rpc.get_new_address(None, None)?;
            let checked_address = new_address.require_network(bitcoin_network)?;
            rpc.generate_to_address(101, &checked_address)?;
            println!(
                "  {} Initial blocks generated. Waiting for balance to be available...",
                "✓".bold().green()
            );
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
            None,
        )?;
        println!(
            "  {} Transaction sent: {}",
            "✓".bold().green(),
            tx.to_string().yellow()
        );
        // Generate a block to confirm the transaction
        let new_address = rpc.get_new_address(None, None)?;
        let checked_new_address = new_address.require_network(bitcoin_network)?;
        rpc.generate_to_address(1, &checked_new_address)?;

        // Wait for transaction confirmation
        loop {
            match rpc.get_transaction(&tx, None) {
                Ok(info) if info.info.confirmations > 0 => {
                    println!(
                        "  {} Transaction confirmed with {} confirmations",
                        "✓".bold().green(),
                        info.info.confirmations.to_string().yellow()
                    );
                    return Ok(Some(info));
                }
                Ok(_) => println!("  {} Waiting for confirmation...", "⏳".bold().blue()),
                Err(e) => println!(
                    "  {} Error checking transaction: {}",
                    "⚠".bold().yellow(),
                    e.to_string().red()
                ),
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    } else {
        println!("{}", "Please deposit funds to continue:".bold());
        println!(
            "  {} Deposit address: {}",
            "→".bold().blue(),
            account_address.yellow()
        );
        println!(
            "  {} Minimum required: {} satoshis",
            "ℹ".bold().blue(),
            "3000".yellow()
        );
        println!("  {} Waiting for funds...", "⏳".bold().blue());

        // TODO: Implement balance checking for non-REGTEST networks
        Ok(None)
    }
}

async fn deploy_program(
    program_keypair: &Keypair,
    program_pubkey: &Pubkey,
    txid: &str,
    vout: u32,
) -> Result<()> {
    // Create a new account for the program
    create_program_account(program_keypair, program_pubkey, txid, vout).await?;

    // Deploy the program transactions
    deploy_program_txs(program_keypair, program_pubkey).await?;

    // Make program executable
    make_program_executable(program_keypair, program_pubkey).await?;

    Ok(())
}

async fn make_program_executable(program_keypair: &Keypair, program_pubkey: &Pubkey) -> Result<()> {
    println!("    Making program executable...");
    let (txid, _) = sign_and_send_instruction_async(
        Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![AccountMeta {
                pubkey: *program_pubkey,
                is_signer: true,
                is_writable: true,
            }],
            data: vec![2],
        },
        vec![*program_keypair],
    )
    .await?;
    println!("    Transaction sent: {}", txid.clone());
    get_processed_transaction_async(NODE1_ADDRESS.to_string(), txid.clone()).await?;
    println!("    Program made executable successfully");
    Ok(())
}
async fn deploy_program_txs(program_keypair: &Keypair, _program_pubkey: &Pubkey) -> Result<()> {
    println!("    Deploying program transactions...");
    deploy_program_txs_async(
        *program_keypair,
        "src/app/program/target/sbf-solana-solana/release/arch_network_app.so",
    )
    .await?;
    println!("    Program transactions deployed successfully");
    Ok(())
}
async fn create_program_account(
    program_keypair: &Keypair,
    program_pubkey: &Pubkey,
    txid: &str,
    vout: u32,
) -> Result<()> {
    println!("    Creating program account...");
    let (txid, _) = sign_and_send_instruction_async(
        SystemInstruction::new_create_account_instruction(
            hex::decode(txid).unwrap().try_into().unwrap(),
            vout,
            *program_pubkey,
        ),
        vec![*program_keypair],
    )
    .await?;
    get_processed_transaction_async(NODE1_ADDRESS.to_string(), txid.clone()).await?;
    println!("    Program account created successfully");
    Ok(())
}

pub async fn demo_start(config: &Config) -> Result<()> {
    println!("{}", "Starting the demo application...".bold().green());

    set_env_vars(config)?;
    
    // Get the demo frontend port from the config or use the default
    let frontend_port = config
        .get_string("demo.frontend_port")
        .unwrap_or_else(|_| "5173".to_string());

    let output = ShellCommand::new("docker-compose")
        .arg("-f")
        .arg("demo-docker-compose.yml")
        .arg("up")
        .arg("-d")// Ensure the images are rebuilt if there are changes
        .env("DEMO_FRONTEND_PORT", &frontend_port)  // Pass the port as an environment variable
        .output()
        .context("Failed to start the demo application using Docker Compose")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to start the demo application: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("{}", "Demo application started successfully!".bold().green());

    // Use the configured frontend port for the demo URL
    let demo_url = format!("http://localhost:{}", frontend_port);

    println!("{}", "You can access the demo application at:".bold());
    println!("  {} {}", "→".bold().blue(), demo_url.yellow());

    // Optionally, open the URL in the default web browser
    if webbrowser::open(&demo_url).is_ok() {
        println!("  {} Opened the demo application in your default web browser", "✓".bold().green());
    } else {
        println!("  {} Unable to open the URL automatically. Please visit it manually.", "ℹ".bold().blue());
    }

    Ok(())
}

pub async fn demo_stop(config: &Config) -> Result<()> {
    println!("{}", "Stopping the demo application...".bold().green());

    set_env_vars(config)?;

    let output = ShellCommand::new("docker-compose")
        .arg("-f")
        .arg("demo-docker-compose.yml")
        .arg("down")
        .output()
        .context("Failed to stop the demo application using Docker Compose")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to stop the demo application: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!(
        "{}",
        "Demo application stopped successfully!".bold().green()
    );
    Ok(())
}

pub async fn config_view(config: &Config) -> Result<()> {
    println!("{}", "Current configuration:".bold().green());
    println!("{:#?}", config);
    Ok(())
}

pub async fn config_edit() -> Result<()> {
    println!("{}", "Editing configuration...".bold().yellow());

    // Get the path to the configuration file
    let config_path = get_config_path()?;

    // Check if the config file exists
    if !config_path.exists() {
        println!(
            "  {} Configuration file not found. Creating a default one...",
            "ℹ".bold().blue()
        );
        config_reset().await?;
    }

    // Get the user's preferred editor
    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "nano".to_string()
            }
        });

    println!(
        "  {} Opening configuration file with {}...",
        "→".bold().blue(),
        editor
    );

    // Open the editor
    let status = Command::new(&editor)
        .arg(&config_path)
        .status()
        .context(format!("Failed to open editor: {}", editor))?;

    if status.success() {
        println!(
            "  {} Configuration file closed. Verifying changes...",
            "✓".bold().green()
        );

        // Attempt to reload the configuration to verify it's still valid
        match Config::builder()
            .add_source(config::File::with_name(config_path.to_str().unwrap()))
            .build()
        {
            Ok(_) => println!(
                "  {} Configuration updated successfully!",
                "✓".bold().green()
            ),
            Err(e) => {
                println!(
                    "  {} Warning: The configuration file may contain errors.",
                    "⚠".bold().yellow()
                );
                println!("    Error details: {}", e);
                println!("    Please review and correct the configuration file.");
            }
        }
    } else {
        println!(
            "  {} Editor closed without saving changes or encountered an error",
            "ℹ".bold().blue()
        );
    }

    Ok(())
}
pub async fn config_reset() -> Result<()> {
    println!(
        "{}",
        "Resetting configuration to default...".bold().yellow()
    );

    let config_path = get_config_path()?;
    let config_dir = config_path.parent().unwrap();

    // Check if the config file already exists
    if config_path.exists() {
        println!(
            "  {} Existing configuration found at {}",
            "ℹ".bold().blue(),
            config_path.display()
        );
        print!(
            "  {} Are you sure you want to overwrite it? (y/N): ",
            "?".bold().yellow()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("  {} Configuration reset cancelled", "ℹ".bold().blue());
            return Ok(());
        }
    }

    // Create the config directory if it doesn't exist
    fs::create_dir_all(config_dir).context("Failed to create config directory")?;

    // Copy the default config to the correct location
    fs::copy("config.default.toml", &config_path).context("Failed to reset configuration")?;

    println!(
        "  {} Configuration reset to default at {}",
        "✓".bold().green(),
        config_path.display()
    );
    Ok(())
}

// Update the create_account function
pub async fn create_account(args: &CreateAccountArgs, config: &Config) -> Result<()> {
    println!("{}", "Creating account for dApp...".bold().green());

    // Get the keys directory
    let keys_dir = ensure_keys_dir()?;
    let accounts_file = keys_dir.join("accounts.json");

    // Check if an account with the same name already exists
    if account_name_exists(&accounts_file, &args.name)? {
        return Err(anyhow!(
            "An account with the name '{}' already exists. Please choose a different name.",
            args.name
        ));
    }

    // Create a new keypair
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
    let caller_keypair = Keypair::from_secret_key(&secp, &secret_key);

    // Convert secp256k1::PublicKey to Pubkey
    let public_key_bytes = public_key.serialize_uncompressed();
    let caller_pubkey = Pubkey::from_slice(&public_key_bytes[1..33]); // Skip the first byte and take the next 32

    // Get account address
    let account_address = generate_account_address(caller_pubkey).await?;

    // Set up Bitcoin RPC client
    let wallet_manager = WalletManager::new(config)?;

    // Prompt user to send funds
    println!("{}", "Please send funds to the following address:".bold());
    println!(
        "  {} Bitcoin address: {}",
        "→".bold().blue(),
        account_address.yellow()
    );
    println!(
        "  {} Minimum required: {} satoshis",
        "ℹ".bold().blue(),
        "3000".yellow()
    );
    println!("  {} Waiting for funds...", "⏳".bold().blue());

    create_arch_account(
        &caller_keypair,
        &caller_pubkey,
        &account_address,
        &wallet_manager,
        config,
    )
    .await?;

    // Determine the program ID to transfer ownership to
    let program_id = if let Some(hex_program_id) = &args.program_id {
        if hex_program_id.is_empty() {
            println!(
                "  {} No program ID provided. Using system program.",
                "ℹ".bold().blue()
            );
            Pubkey::system_program()
        } else {
            let program_id_bytes =
                hex::decode(hex_program_id).context("Failed to decode program ID from hex")?;
            Pubkey::from_slice(&program_id_bytes)
        }
    } else {
        println!(
            "  {} No program ID provided. Using system program.",
            "ℹ".bold().blue()
        );
        Pubkey::system_program()
    };

    // Transfer ownership to the program
    transfer_account_ownership(&caller_keypair, &caller_pubkey, &program_id).await?;

    // Save the account information to accounts.json
    save_account_to_file(&accounts_file, &secret_key, &public_key, &args.name)?;

    // Output the private key to the user
    let private_key_hex = hex::encode(secret_key.secret_bytes());
    println!(
        "{}",
        "Account created and ownership transferred successfully!"
            .bold()
            .green()
    );
    println!(
        "{}",
        "IMPORTANT: Please save your private key securely. It will not be displayed again."
            .bold()
            .red()
    );
    println!(
        "  {} Private Key: {}",
        "🔑".bold().yellow(),
        private_key_hex.bright_red()
    );
    println!(
        "  {} Public Key: {}",
        "🔑".bold().yellow(),
        hex::encode(public_key.serialize()).bright_green()
    );

    // Close the Bitcoin wallet
    wallet_manager.close_wallet()?;

    Ok(())
}

fn account_name_exists(accounts_file: &Path, name: &str) -> Result<bool> {
    if !accounts_file.exists() {
        return Ok(false);
    }

    let file = OpenOptions::new().read(true).open(accounts_file)?;
    let reader = BufReader::new(file);
    let accounts: Value = serde_json::from_reader(reader)?;

    for account_info in accounts.as_object().unwrap().values() {
        if account_info["name"].as_str().unwrap() == name {
            return Ok(true);
        }
    }

    Ok(false)
}

fn save_account_to_file(
    file_path: &Path,
    secret_key: &SecretKey,
    public_key: &secp256k1::PublicKey,
    name: &str,
) -> Result<()> {
    let mut accounts = if file_path.exists() {
        let file = OpenOptions::new().read(true).open(file_path)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader)?
    } else {
        json!({})
    };

    let account_id = hex::encode(public_key.serialize());
    let private_key = hex::encode(secret_key.secret_bytes());

    accounts[&account_id] = json!({
        "name": name,
        "private_key": private_key,
        "public_key": hex::encode(public_key.serialize()),
    });

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;
    serde_json::to_writer_pretty(file, &accounts)?;

    println!(
        "  {} Account '{}' saved to {}",
        "✓".bold().green(),
        name,
        file_path.display()
    );
    Ok(())
}

// Add a new function to list accounts
pub async fn list_accounts() -> Result<()> {
    let keys_dir = ensure_keys_dir()?;
    let accounts_file = keys_dir.join("accounts.json");

    if !accounts_file.exists() {
        println!("  {} No accounts found", "ℹ".bold().blue());
        return Ok(());
    }

    let file = OpenOptions::new().read(true).open(accounts_file)?;
    let reader = BufReader::new(file);
    let accounts: Value = serde_json::from_reader(reader)?;

    println!("{}", "Stored accounts:".bold().green());
    for (account_id, account_info) in accounts.as_object().unwrap() {
        println!(
            "  {} Account: {}",
            "→".bold().blue(),
            account_info["name"].as_str().unwrap().yellow()
        );
        println!("    ID: {}", account_id);
        println!(
            "    Public Key: {}",
            account_info["public_key"].as_str().unwrap()
        );
    }

    Ok(())
}

pub async fn delete_account(args: &DeleteAccountArgs) -> Result<()> {
    let keys_dir = ensure_keys_dir()?;
    let accounts_file = keys_dir.join("accounts.json");

    if !accounts_file.exists() {
        println!("  {} No accounts found", "ℹ".bold().blue());
        return Ok(());
    }

    let file = OpenOptions::new().read(true).open(&accounts_file)?;
    let reader = BufReader::new(file);
    let mut accounts: Value = serde_json::from_reader(reader)?;

    let accounts_obj = accounts.as_object_mut().unwrap();
    let mut account_to_remove = None;

    for (account_id, account_info) in accounts_obj.iter() {
        if account_id == &args.identifier
            || account_info["name"].as_str().unwrap() == args.identifier
        {
            account_to_remove = Some(account_id.clone());
            break;
        }
    }

    if let Some(account_id) = account_to_remove {
        println!(
            "  {} Account '{}' found. Are you sure you want to delete it? (yes/no)",
            "ℹ".bold().blue(),
            args.identifier
        );
        let mut response = String::new();
        std::io::stdin().read_line(&mut response)?;
        if response.trim().to_lowercase() == "yes" {
            accounts_obj.remove(&account_id);
            let file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&accounts_file)?;
            serde_json::to_writer_pretty(file, &accounts)?;
            println!(
                "  {} Account '{}' deleted successfully",
                "✓".bold().green(),
                args.identifier
            );
        } else {
            println!(
                "  {} Deletion of account '{}' cancelled",
                "✗".bold().red(),
                args.identifier
            );
        }
    } else {
        println!(
            "  {} Account '{}' not found",
            "✗".bold().red(),
            args.identifier
        );
    }

    Ok(())
}

pub fn get_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Failed to determine config directory"))?
        .join("arch-cli");

    fs::create_dir_all(&config_dir)?;

    Ok(config_dir)
}

pub fn ensure_keys_dir() -> Result<PathBuf> {
    let keys_dir = get_config_dir()?.join("keys");
    fs::create_dir_all(&keys_dir)?;
    Ok(keys_dir)
}

async fn generate_account_address(caller_pubkey: Pubkey) -> Result<String> {
    // Get program account address from network
    let account_address = get_account_address_async(caller_pubkey)
        .await
        .context("Failed to get account address")?;
    // println!("  {} Account address: {}", "ℹ".bold().blue(), account_address.yellow());

    Ok(account_address)
}

async fn _wait_for_funds(client: &Client, address: &str, config: &Config) -> Result<()> {
    // Check if wallet_manager.client is connected
    let connected = client.get_blockchain_info()?;
    println!("  {} Connected: {:?}", "ℹ".bold().blue(), connected);

    let tx_info = fund_address(client, address, config).await?;

    if let Some(info) = tx_info {
        println!(
            "  {} Transaction confirmed with {} confirmations",
            "✓".bold().green(),
            info.info.confirmations.to_string().yellow()
        );
    }

    Ok(())
}

async fn create_arch_account(
    caller_keypair: &Keypair,
    caller_pubkey: &Pubkey,
    account_address: &str,
    wallet_manager: &WalletManager,
    config: &Config,
) -> Result<()> {
    let tx_info = fund_address(&wallet_manager.client, account_address, config).await?;

    // Output the bitcoin transaction info
    // println!("  {} Transaction info: {:?}", "ℹ".bold().blue(), tx_info);

    if let Some(info) = tx_info {
        let (txid, _) = sign_and_send_instruction_async(
            SystemInstruction::new_create_account_instruction(
                hex::decode(&info.info.txid.to_string())
                    .unwrap()
                    .try_into()
                    .unwrap(),
                0,
                *caller_pubkey,
            ),
            vec![*caller_keypair],
        )
        .await
        .expect("signing and sending a transaction should not fail");

        println!(
            "  {} Account created with Arch Network transaction ID: {}",
            "✓".bold().green(),
            txid.yellow()
        );
        Ok(())
    } else {
        println!(
            "  {} Warning: No transaction info available for deployment",
            "⚠".bold().yellow()
        );
        // You might want to implement an alternative deployment method for non-REGTEST networks
        Ok(())
    }
}

async fn transfer_account_ownership(
    caller_keypair: &Keypair,
    account_pubkey: &Pubkey,
    program_pubkey: &Pubkey,
) -> Result<()> {
    let mut instruction_data = vec![3]; // Transfer instruction
    instruction_data.extend(program_pubkey.serialize());

    println!(
        "  {} Account public key: {:?}",
        "ℹ".bold().blue(),
        hex::encode(account_pubkey.serialize())
    );

    let (_txid, _) = sign_and_send_instruction_async(
        Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![AccountMeta {
                pubkey: *account_pubkey,
                is_signer: true,
                is_writable: true,
            }],
            data: instruction_data,
        },
        vec![*caller_keypair],
    )
    .await
    .expect("signing and sending a transaction should not fail");

    Ok(())
}

pub async fn indexer_start(config: &Config) -> Result<()> {
    println!("{}", "Starting the arch-indexer...".bold().green());

    set_env_vars(config)?;

    let output = ShellCommand::new("docker-compose")
        .arg("-f")
        .arg("./arch-indexer/docker-compose.yml") // Updated path
        .arg("up")
        .arg("-d")
        .output()
        .context("Failed to start the arch-indexer using Docker Compose")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to start the arch-indexer: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("{}", "arch-indexer started successfully!".bold().green());
    Ok(())
}

pub async fn indexer_stop(config: &Config) -> Result<()> {
    println!("{}", "Stopping the arch-indexer...".bold().green());

    set_env_vars(config)?;

    let output = ShellCommand::new("docker-compose")
        .arg("-f")
        .arg("./arch-indexer/docker-compose.yml")
        .arg("down")
        .output()
        .context("Failed to stop the arch-indexer using Docker Compose")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to stop the arch-indexer: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("{}", "arch-indexer stopped successfully!".bold().green());
    Ok(())
}

pub async fn validator_start(args: &ValidatorStartArgs) -> Result<()> {
    println!("{}", "Starting the local validator...".bold().green());

    let network = &args.network;
    let rust_log = "info";
    let rpc_bind_ip = "0.0.0.0";
    let rpc_bind_port = "9002";
    let bitcoin_rpc_endpoint = "bitcoin-node.dev.aws.archnetwork.xyz";
    let bitcoin_rpc_port = "18443";
    let bitcoin_rpc_username = "bitcoin";
    let bitcoin_rpc_password = "428bae8f3c94f8c39c50757fc89c39bc7e6ebc70ebf8f618";

    let container_name = "local_validator";
    let container_exists = String::from_utf8(
        ShellCommand::new("docker")
            .arg("ps")
            .arg("-a")
            .arg("--format")
            .arg("{{.Names}}")
            .output()
            .context("Failed to check existing containers")?
            .stdout
    )?
    .lines()
    .any(|name| name == container_name);

    let output = if container_exists {
        ShellCommand::new("docker")
            .arg("start")
            .arg(container_name)
            .output()
            .context("Failed to start the existing local validator container")?
    } else {
        ShellCommand::new("docker")
        .arg("run")
        .arg("--platform")
        .arg("linux/amd64")
        .arg("--rm")
        .arg("-d")
        .arg("--name")
        .arg("local_validator")
        .arg("-e")
        .arg(format!("RUST_LOG={}", rust_log))
        .arg("-p")
        .arg(format!("{}:{}", rpc_bind_port, rpc_bind_port))
        .arg("ghcr.io/arch-network/local_validator:pr-282")
        .arg("/usr/bin/local_validator")
        .arg("--rpc-bind-ip")
        .arg(rpc_bind_ip)
        .arg("--rpc-bind-port")
        .arg(rpc_bind_port)
        .arg("--bitcoin-rpc-endpoint")
        .arg(bitcoin_rpc_endpoint)
        .arg("--bitcoin-rpc-port")
        .arg(bitcoin_rpc_port)
        .arg("--bitcoin-rpc-username")
        .arg(bitcoin_rpc_username)
        .arg("--bitcoin-rpc-password")
        .arg(bitcoin_rpc_password)
        .output()
        .context("Failed to start the local validator")?
    };

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to start the local validator: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("{}", "Local validator started successfully!".bold().green());
    Ok(())
}

pub async fn validator_stop() -> Result<()> {
    println!("{}", "Stopping the local validator...".bold().green());

    let output = ShellCommand::new("docker")
        .arg("stop")
        .arg("local_validator")
        .output()
        .context("Failed to stop the local validator")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to stop the local validator: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("{}", "Local validator stopped successfully!".bold().green());
    Ok(())
}
