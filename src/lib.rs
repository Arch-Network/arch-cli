use arch_program::account::AccountMeta;
use arch_program::instruction::Instruction;
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction::SystemInstruction;
use bitcoin::Amount;
use bitcoin::Network;
use bitcoincore_rpc::jsonrpc::client;
use bitcoincore_rpc::jsonrpc::serde_json;
use clap::{ Parser, Subcommand, Args };
use common::wallet_manager;
use secp256k1::Keypair;
use serde::Deserialize;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use tokio;
use anyhow::{ Context, Result };
use std::process::Command as ShellCommand;
use common::helper::*;
use common::constants::*;
use bitcoin::Address;
use bitcoincore_rpc::{ Client, RpcApi };
use std::time::Duration;
use std::str::FromStr;
use colored::*;
use std::process::Command;
use config::{ Config, File, Environment };
use std::env;
use anyhow::anyhow;

use common::wallet_manager::*;
use std::process::Stdio;
use tokio::process::Command as TokioCommand;
use tokio::io::AsyncBufReadExt;
use webbrowser;

#[derive(Deserialize)]
pub struct ServiceConfig {
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
    #[clap(long_about = "Creates the project structure and boilerplate files for a new Arch Network application.")]
    Init,

    /// Manage the development server
    #[clap(subcommand)]
    Server(ServerCommands),

    /// Deploy your Arch Network app
    #[clap(long_about = "Builds and deploys your Arch Network application to the specified network.")]
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
    Frontend(FrontendCommands),

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
}

#[derive(Subcommand)]
pub enum ServerCommands {
    /// Start the development server
    #[clap(long_about = "Starts the development environment, including Bitcoin regtest network and Arch Network nodes.")]
    Start,

    /// Stop the development server
    #[clap(long_about = "Stops all related Docker containers and services for the development environment.")]
    Stop,

    /// Check the status of the development server
    #[clap(long_about = "Displays the current status of all services in the development environment.")]
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
pub enum FrontendCommands {
    /// Start the frontend application
    #[clap(long_about = "Prepares and starts the frontend application, opening it in the default browser.")]
    Start,
}

#[derive(Subcommand)]
pub enum AccountCommands {
    /// Create an account for the dApp
    #[clap(long_about = "Creates an account for the dApp, prompts for funding, and transfers ownership to the program")]
    Create(CreateAccountArgs),
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
}

#[derive(Args)]
pub struct DeployArgs {
    /// Directory of your program
    #[clap(long, help = "Specifies the directory containing your Arch Network program")]
    directory: Option<String>,

    /// Path to the program key file
    #[clap(long, help = "Specifies the path to the program's key file for deployment")]
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

pub async fn init() -> Result<()> {
    println!("{}", "Initializing new Arch Network app...".bold().green());

    // Check dependencies
    check_dependencies()?;

    // Navigate to the program folder and run `cargo build-sbf`
    println!("{}", "Building Arch Network program...".bold().blue());
    ShellCommand::new("cargo")
        .current_dir("program")
        .arg("build-sbf")
        .output()
        .expect("Failed to build Arch Network program");

    // Create project structure
    println!("{}", "Creating project structure...".bold().blue());
    let dirs = ["src/app/backend", "src/app/keys"];
    for dir in dirs.iter() {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create directory: {}", dir.yellow()))?;
    }

    // Create boilerplate files
    println!("{}", "Creating boilerplate files...".bold().blue());
    let files = [
        ("src/app/backend/index.ts", include_str!("templates/backend_index.ts")),
        ("src/app/backend/package.json", include_str!("templates/backend_package.json")),
    ];

    for (file_path, content) in files.iter() {
        if !Path::new(file_path).exists() {
            fs::write(file_path, content)
                .with_context(|| format!("Failed to write file: {}", file_path))?;
        } else {
            println!("  {} File already exists, skipping: {}", "ℹ".bold().blue(), file_path);
        }
    }

    // Check if program and frontend directories exist
    let program_dir = Path::new("src/app/program");
    let frontend_dir = Path::new("src/app/frontend");

    if !program_dir.exists() {
        println!("  {} Creating default program directory", "→".bold().blue());
        fs::create_dir_all(program_dir)?;
        fs::write(
            program_dir.join("src/lib.rs"),
            include_str!("templates/program_lib.rs")
        )?;
        fs::write(
            program_dir.join("Cargo.toml"),
            include_str!("templates/program_cargo.toml")
        )?;
    } else {
        println!("  {} Existing program directory found, preserving it", "ℹ".bold().blue());
    }

    if !frontend_dir.exists() {
        println!("  {} Creating default frontend directory", "→".bold().blue());
        fs::create_dir_all(frontend_dir)?;
        fs::write(
            frontend_dir.join("index.html"),
            include_str!("templates/frontend_index.html")
        )?;
        fs::write(
            frontend_dir.join("index.js"),
            include_str!("templates/frontend_index.js")
        )?;
        fs::write(
            frontend_dir.join("package.json"),
            include_str!("templates/frontend_package.json")
        )?;
    } else {
        println!("  {} Existing frontend directory found, preserving it", "ℹ".bold().blue());
    }

    println!("  {} New Arch Network app initialized successfully!", "✓".bold().green());
    Ok(())
}

fn get_docker_compose_command() -> (&'static str, &'static [&'static str]) {
    if Command::new("docker-compose").arg("--version").output().is_ok() {
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
            match Command::new(command[0])
                .args(&command[1..])
                .output()
            {
                Ok(output) if output.status.success() => {
                    version = String::from_utf8_lossy(&output.stdout).to_string();
                    success = true;
                    break;
                },
                _ => continue,
            }
        }

        if success {
            println!(" {}", "✓".bold().green());
            println!("    Detected version: {}", version.trim());

            // Additional check for Node.js version
            if *name == "node" {
                let version_str = version.split('v').nth(1).unwrap_or("").trim();
                let major_version = version_str.split('.').next().unwrap_or("0").parse::<u32>().unwrap_or(0);
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

    println!("{}", "All required dependencies are installed.".bold().green());
    Ok(())
}

fn start_or_create_services(service_name: &str, service_config: &ServiceConfig) -> Result<()> {
    println!("  {} Starting {}...", "→".bold().blue(), service_name.yellow());

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
                .args(&["-f", &service_config.docker_compose_file, "start"])
                .output()
                .context(format!("Failed to start existing {} containers", service_name))?;

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
            .args(&["--progress", "auto", "-f", &service_config.docker_compose_file, "up", "--build", "-d"])
            .envs(std::env::vars())
            .output()
            .context(format!("Failed to create and start {} containers", service_name))?;

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
            return Err(anyhow!("Failed to create and start {} containers", service_name));
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
    println!("{}", "Starting development server...".bold().green());

    set_env_vars(config)?;

    let network_type = config
        .get_string("network.type")
        .context("Failed to get network type from configuration")?;
    
    let mut server_started_successfully = true;
    if network_type == "development" {
        // set_env_vars(config)?;
        create_docker_network("arch-network")?;

        let bitcoin_config: ServiceConfig = config
            .get("bitcoin")
            .context("Failed to get Bitcoin configuration")?;
        if let Err(e) = start_or_create_services("Bitcoin regtest network", &bitcoin_config) {
            println!("  ⚠ Warning: {}", e);
            server_started_successfully = false;
        }

        let arch_config: ServiceConfig = config
            .get("arch")
            .context("Failed to get Arch Network configuration")?;
        if let Err(e) = start_or_create_services("Arch Network nodes", &arch_config) {
            println!("  ⚠ Warning: {}", e);
            server_started_successfully = false;
        }
    } else {
        println!(
            "  {} Using existing network configuration for: {}",
            "ℹ".bold().blue(),
            network_type.yellow()
        );
    }

    if server_started_successfully {
        println!("  {} Development server started successfully!", "✓".bold().green());
    } else {
        println!("  ⚠ Development server encountered issues during startup.");
    }

    Ok(())
}

pub async fn deploy(args: &DeployArgs, config: &Config) -> Result<()> {
    println!("{}", "Deploying your Arch Network app...".bold().green());

    // Build the program
    build_program(args).context("Failed to build program")?;
    println!("{}", "Program built successfully".bold().green());

    // Get program key and public key
    let program_key_path = get_program_key_path(args, config)?;
    let (program_keypair, program_pubkey) = with_secret_key_file(&program_key_path)
        .context("Failed to get program key pair")?;
    println!("  {} Program keypair: {:?}", "ℹ".bold().blue(), program_keypair);

    println!("  {} Program public key: {}", "ℹ".bold().blue(), program_pubkey);

    // Get program account address from network
    let account_address = get_account_address_async(program_pubkey)
        .await
        .context("Failed to get account address")?;
    println!("  {} Account address: {}", "ℹ".bold().blue(), account_address.yellow());

    // Set up Bitcoin RPC client
    let wallet_manager = WalletManager::new(config)?;

    // Check if wallet_manager.client is connected
    let connected = wallet_manager.client.get_blockchain_info()?;

    let balance = wallet_manager.client.get_balance(None, None)?;

    println!("  {} Balance: {}", "ℹ".bold().blue(), balance);
    if balance == Amount::ZERO {
        println!("  {} Generating initial blocks to receive mining rewards...", "→".bold().blue());
        let new_address = wallet_manager.client.get_new_address(None, None)?;
        let checked_address = new_address.require_network(Network::Regtest)?;
        wallet_manager.client.generate_to_address(101, &checked_address)?;
        println!("  {} Initial blocks generated. Waiting for balance to be available...", "✓".bold().green());
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // Handle fund transfer based on network type
    let tx_info = fund_address(&wallet_manager.client, &account_address, config).await?;

    // Deploy the program
    deploy_program_with_tx_info(&program_keypair, &program_pubkey, tx_info).await?;

    wallet_manager.close_wallet()?;

    println!("{}", "Your app has been deployed successfully!".bold().green());
    Ok(())
}

pub async fn server_stop() -> Result<()> {
    println!("{}", "Stopping development server...".bold().yellow());

    stop_all_related_containers()?;

    println!("{}", "Development server stopped successfully!".bold().green());
    println!("{}", "You can restart the server later using the 'server start' command.".italic());
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
    wallet_manager.client.generate_to_address(1, &address_networked)?;

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
        println!("  {} Stopping {} containers...", "→".bold().blue(), prefix.yellow());

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
    println!("  {} Fetching logs for {}...", "→".bold().blue(), service_name.yellow());

    for container in &service_config.services {
        println!("    Logs for {}:", container.bold());
        let log_output = Command::new("docker")
            .args(&["logs", "--tail", "50", container])
            .output()
            .context(format!("Failed to fetch logs for container {}", container))?;

        println!("{}", String::from_utf8_lossy(&log_output.stdout));
    }

    Ok(())
}

fn check_service_status(service_name: &str, service_config: &ServiceConfig) -> Result<()> {
    println!("  {} Checking {} status...", "→".bold().blue(), service_name.yellow());

    for container in &service_config.services {
        let status_output = Command::new("docker")
            .args(&["ps", "-a", "--filter", &format!("name={}", container), "--format", "{{.Status}}"])
            .output()
            .context(format!("Failed to check status of container {}", container))?;

        let status = String::from_utf8_lossy(&status_output.stdout).trim().to_string();

        if status.starts_with("Up") {
            println!("    {} {} is running", "✓".bold().green(), container);
        } else if status.is_empty() {
            println!("    {} {} is not created", "✗".bold().red(), container);
        } else {
            println!("    {} {} is not running (status: {})", "✗".bold().red(), container, status);
        }
    }

    Ok(())
}

pub async fn server_logs(service: &str, config: &Config) -> Result<()> {
    println!("{}", format!("Fetching logs for {}...", service).bold().blue());

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
        .args(&["-f", compose_file, "ps", "-q"])
        .output()
        .context("Failed to list existing containers")?;

    if !output.stdout.is_empty() {
        println!("  {} Found existing containers. Starting them...", "→".bold().blue());
        let start_output = Command::new("docker-compose")
            .args(&["-f", compose_file, "start"])
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
            println!("  {} Existing containers started successfully.", "✓".bold().green());
        }
    } else {
        println!("  {} No existing containers found. Creating new ones...", "ℹ".bold().blue());
        // Proceed with your existing logic to create new containers
    }

    Ok(())
}

pub fn remove_docker_networks() -> Result<()> {
    let networks = vec!["arch-network", "internal"];

    for network in networks {
        println!("  {} Removing Docker network: {}", "→".bold().blue(), network.yellow());

        let output = Command::new("docker")
            .args(&["network", "rm", network])
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
            println!("  {} Network {} removed successfully.", "✓".bold().green(), network.yellow());
        }
    }

    Ok(())
}

pub fn stop_docker_services(compose_file: &str, service_name: &str) -> Result<()> {
    println!("  {} Stopping {} services...", "→".bold().blue(), service_name.yellow());
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();
    
    let output = Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(&["-f", compose_file, "down"])
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
    println!("{}", "Cleaning project...".bold().yellow());

    fs::remove_dir_all("src/app").context("Failed to remove src/app directory")?;
    fs::remove_dir_all("arch-data").context("Failed to remove arch-data directory")?;

    println!("  {} Project cleaned successfully!", "✓".bold().green());
    Ok(())
}

pub fn start_bitcoin_regtest() -> Result<()> {
    println!("  {} Starting Bitcoin regtest network...", "→".bold().blue());
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();
    
    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(&["-f", "path/to/bitcoin-docker-compose.yml", "up", "-d"])
        .status()?;
    
    println!("  {} Bitcoin regtest network started successfully.", "✓".bold().green());
    Ok(())
}

pub fn stop_bitcoin_regtest() -> Result<()> {
    println!("  {} Stopping Bitcoin regtest network...", "→".bold().blue());
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();
    
    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(&["-f", "path/to/bitcoin-docker-compose.yml", "down"])
        .status()?;
    
    println!("  {} Bitcoin regtest network stopped successfully.", "✓".bold().green());
    Ok(())
}

pub async fn start_dkg(config: &Config) -> Result<()> {
    println!("{}", "Starting Distributed Key Generation (DKG) process...".bold().green());

    let leader_rpc = config
        .get_string("arch.leader_rpc_endpoint")
        .expect("Failed to get leader RPC endpoint from config");

    // Create an HTTP client
    let client = reqwest::Client::new();

    // Prepare the RPC request
    let rpc_request =
        serde_json::json!({
        "jsonrpc": "2.0",
        "method": "start_dkg",
        "params": [],
        "id": 1
    });

    // Send the RPC request
    let response = client
        .post(&leader_rpc)
        .json(&rpc_request)
        .send().await
        .context("Failed to send RPC request")?;

    // Check the response
    if response.status().is_success() {
        let result: serde_json::Value = response
            .json().await
            .context("Failed to parse JSON response")?;
        println!("  {} DKG process started successfully", "✓".bold().green());
        println!(
            "  {} Response: {}",
            "ℹ".bold().blue(),
            serde_json::to_string_pretty(&result).unwrap()
        );
    } else {
        let error_message = response.text().await.context("Failed to get error message")?;
        println!("  {} Failed to start DKG process", "✗".bold().red());
        println!("  {} Error: {}", "ℹ".bold().blue(), error_message);
    }

    Ok(())
}

pub fn start_arch_nodes() -> Result<()> {
    println!("  {} Starting Arch Network nodes...", "→".bold().blue());
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();
    
    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(&["-f", "path/to/arch-docker-compose.yml", "up", "-d"])
        .status()?;
    
    println!("  {} Arch Network nodes started successfully.", "✓".bold().green());
    Ok(())
}

pub fn stop_arch_nodes() -> Result<()> {
    println!("  {} Stopping Arch Network nodes...", "→".bold().blue());
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();
    
    Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(&["-f", "path/to/arch-docker-compose.yml", "down"])
        .status()?;
    
    println!("  {} Arch Network nodes stopped successfully.", "✓".bold().green());
    Ok(())
}

pub fn load_config() -> Result<Config> {
    let config_path = "config.toml";

    let mut builder = Config::builder();

    // Check if the config file exists
    if Path::new(config_path).exists() {
        builder = builder.add_source(File::with_name(config_path));
        println!("  {} Loading configuration from {}", "→".bold().blue(), config_path.yellow());
    } else {
        println!(
            "  {} Warning: {} not found. Using default configuration.",
            "⚠".bold().yellow(),
            config_path.yellow()
        );
        // You might want to create a default config here
    }

    // Add environment variables as a source (this will override file settings)
    builder = builder.add_source(Environment::default());

    // Build the configuration
    let config = builder.build().context("Failed to build configuration")?;

    Ok(config)
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

pub fn start_docker_service(service_name: &str, container_name: &str, compose_file: &str) -> Result<()> {
    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();
    
    let is_running = check_docker_status(container_name)?;

    if !is_running {
        let output = Command::new(docker_compose_cmd)
            .args(docker_compose_args)
            .args(&["-f", compose_file, "up", "-d"])
            .output()?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to start {}: {}", service_name, error_message));
        }
        println!("  {} {} started.", "✓".bold().green(), service_name.yellow());
    } else {
        println!("  {} {} already running.", "ℹ".bold().blue(), service_name.yellow());
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

fn create_docker_network(network_name: &str) -> Result<()> {
    let output = Command::new("docker")
        .args(&["network", "create", "--driver", "bridge", network_name])
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
            return Err(anyhow::anyhow!("Failed to create network: {}", error_message));
        }
    } else {
        println!("  {} Created Docker network: {}", "✓".bold().green(), network_name.yellow());
    }

    Ok(())
}

fn build_program(args: &DeployArgs) -> Result<()> {
    let path = args.directory.as_ref().map_or_else(
        || "src/app/program/Cargo.toml".to_string(),
        |dir| format!("{}/Cargo.toml", dir)
    );

    if !std::path::Path::new(&path).exists() {
        println!("{}", "Oops! We couldn't find your Cargo.toml file.".bold().red());
        println!("{}", "Here's what you can do:".bold().yellow());
        println!("  1. Make sure you're in the correct directory");
        println!("  2. Check if the file exists at: {}", path.bold());
        println!("  3. If you've moved your project, update the path in your configuration");
        return Err(anyhow::anyhow!("Cargo.toml not found at: {}", path));
    }

    println!("  {} Building program...", "→".bold().blue());
    let output = std::process::Command::new("cargo")
        .args(&["build-sbf", "--manifest-path", &path])
        .output()
        .context("Failed to execute cargo build-sbf")?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        println!("{}", "Uh-oh! The build process for your Arch program encountered an error.".bold().red());
        println!("{}", "Don't worry, here are some steps to troubleshoot:".bold().yellow());
        println!("  1. Check your code for any syntax errors");
        println!("  2. Ensure all dependencies are correctly specified in Cargo.toml");
        println!("  3. Make sure you have the latest version of the Solana Build Tools installed");
        println!("  4. Try running 'cargo clean' and then deploy again");
        println!("\nError details:");
        println!("{}", error_message);
        return Err(anyhow::anyhow!("Build failed: {}", error_message));
    }

    println!("  {} Program built successfully", "✓".bold().green());
    println!("{}", "Great job! Your program is ready to deploy.".bold().green());
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

async fn fund_address(
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
            None
        )?;
        println!("  {} Transaction sent: {}", "✓".bold().green(), tx.to_string().yellow());
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
                Err(e) =>
                    println!(
                        "  {} Error checking transaction: {}",
                        "⚠".bold().yellow(),
                        e.to_string().red()
                    ),
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
        deploy_program(
            program_keypair,
            program_pubkey,
            &info.info.txid.to_string(),
            0
        ).await?;
        println!("  {} Program deployed successfully", "✓".bold().green());
        Ok(())
    } else {
        println!("  {} Warning: No transaction info available for deployment", "⚠".bold().yellow());
        // You might want to implement an alternative deployment method for non-REGTEST networks
        Ok(())
    }
}

async fn deploy_program(
    program_keypair: &bitcoin::secp256k1::Keypair,
    program_pubkey: &arch_program::pubkey::Pubkey,
    txid: &str,
    vout: u32
) -> Result<(), anyhow::Error> {
    println!("  {} Deploying program...", "→".bold().blue());
    // println!("  {} Program ID: {}", "ℹ".bold().blue(), program_pubkey.to_string().yellow());

    // Check if the program account already exists
    // let account_info = read_account_info_async(NODE1_ADDRESS.to_string(), *program_pubkey).await;
    // if account_info.is_ok() {
    //     println!("  {} Program account already exists", "ℹ".bold().blue());
    //     return Ok(());
    // }

    // 1. Create a new account for the program
    let (txid, _instruction_hash) = sign_and_send_instruction_async(
        SystemInstruction::new_create_account_instruction(
            hex::decode(txid).unwrap().try_into().unwrap(),
            vout,
            *program_pubkey
        ),
        vec![*program_keypair]
    ).await.expect("signing and sending a transaction should not fail");

    get_processed_transaction_async(NODE1_ADDRESS.to_string(), txid.clone()).await.expect(
        "get processed transaction should not fail"
    );

    // 2. Deploy the program transactions
    deploy_program_txs_async(
        *program_keypair,
        "src/app/program/target/sbf-solana-solana/release/arch_network_app.so"
    ).await.expect("deploy program txs should not fail");
    println!("Program transactions deployed successfully");

    // 3. Read the account info
    let elf = fs
        ::read("src/app/program/target/sbf-solana-solana/release/arch_network_app.so")
        .expect("elf path should be available");
    assert!(read_account_info_async(NODE1_ADDRESS.to_string(), *program_pubkey).await.unwrap().data == elf);
    println!("Program account created successfully");

    // 4. Make program executable
    println!("Making program executable");
    let (txid, _instruction_hash) = sign_and_send_instruction_async(
        Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![AccountMeta {
                pubkey: *program_pubkey,
                is_signer: true,
                is_writable: true,
            }],
            data: vec![2],
        },
        vec![*program_keypair]
    ).await.expect("signing and sending a transaction should not fail");

    let processed_tx = get_processed_transaction_async(
        NODE1_ADDRESS.to_string(),
        txid.clone()
    ).await.expect("get processed transaction should not fail");
    // println!("processed_tx {:?}", processed_tx);
    println!("Program made executable successfully");

    assert!(
        read_account_info_async(
            NODE1_ADDRESS.to_string(),
            *program_pubkey
        ).await.unwrap().is_executable
    );

    let program_id_hex = hex::encode(program_pubkey.serialize());
    println!(
        "  {} Program deployed successfully with ID: {}",
        "✓".bold().green(),
        program_id_hex.yellow()
    );
    println!(
        "  {} Use this program ID in your frontend application:",
        "ℹ".bold().blue()
    );
    println!("    {}", program_id_hex.bold());

    println!(
        "  {} Program deployed with transaction ID: {} and vout: {}",
        "✓".bold().green(),
        txid.yellow(),
        vout
    );
    Ok(())
}

// Add this new async function to handle the StartApp command
pub async fn frontend_start() -> Result<()> {
    println!("{}", "Starting the frontend application...".bold().green());

    // Copy .env.example to .env
    println!("  {} Copying .env.example to .env...", "→".bold().blue());
    fs::copy(
        "src/app/frontend/.env.example",
        "src/app/frontend/.env",
    ).context("Failed to copy .env.example to .env")?;
    println!("  {} .env file created", "✓".bold().green());

    // Install npm packages
    println!("  {} Installing npm packages...", "→".bold().blue());
    let npm_install = TokioCommand::new("npm")
        .current_dir("src/app/frontend")
        .arg("install")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .context("Failed to run npm install")?;

    if !npm_install.success() {
        return Err(anyhow!("npm install failed"));
    }
    println!("  {} npm packages installed", "✓".bold().green());

    // Build and start the Vite server
    println!("  {} Building and starting the Vite server...", "→".bold().blue());
    let mut vite_dev = TokioCommand::new("npm")
        .current_dir("src/app/frontend")
        .arg("run")
        .arg("dev")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start Vite server")?;

    // Read the output to get the local URL
    let stdout = vite_dev.stdout.take().expect("Failed to capture stdout");
    let mut reader = tokio::io::BufReader::new(stdout).lines();

    let mut local_url = String::new();
    while let Some(line) = reader.next_line().await? {
        if line.contains("Local:") {
            local_url = line.split("Local:").nth(1).unwrap_or("").trim().to_string();
            break;
        }
    }

    if local_url.is_empty() {
        return Err(anyhow!("Failed to get local URL from Vite server output"));
    }

    println!("  {} Vite server started", "✓".bold().green());

    // Open the browser
    println!("  {} Opening application in default browser...", "→".bold().blue());
    if webbrowser::open(&local_url).is_ok() {
        println!("  {} Application opened in default browser", "✓".bold().green());
    } else {
        println!("  {} Failed to open browser. Please navigate to {} manually", "⚠".bold().yellow(), local_url);
    }

    println!("{}", "Frontend application started successfully!".bold().green());
    println!("Press Ctrl+C to stop the server and exit.");

    // Wait for the Vite process to finish (i.e., until the user interrupts it)
    vite_dev.wait().await?;

    Ok(())
}

pub async fn config_view(config: &Config) -> Result<()> {
    println!("{}", "Current configuration:".bold().green());
    println!("{:#?}", config);
    Ok(())
}

pub async fn config_edit(config: &Config) -> Result<()> {
    println!("{}", "Editing configuration...".bold().yellow());
    // Implement config editing logic here
    // For example, you could open the config file in the user's default text editor
    let editor = env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let status = Command::new(editor)
        .arg("config.toml")
        .status()
        .context("Failed to open editor")?;

    if status.success() {
        println!("  {} Configuration updated successfully!", "✓".bold().green());
    } else {
        println!("  {} Failed to update configuration", "✗".bold().red());
    }
    Ok(())
}

pub async fn config_reset() -> Result<()> {
    println!("{}", "Resetting configuration to default...".bold().yellow());
    // Implement config reset logic here
    // For example, you could copy a default config file over the existing one
    fs::copy("config.default.toml", "config.toml").context("Failed to reset configuration")?;
    println!("  {} Configuration reset to default", "✓".bold().green());
    Ok(())
}

pub async fn create_account(args: &CreateAccountArgs, config: &Config) -> Result<()> {
    println!("{}", "Creating account for dApp...".bold().green());

    // Get caller key
    let caller_key_path = CALLER_FILE_PATH.to_string();
    let (caller_keypair, caller_pubkey) = with_secret_key_file(&caller_key_path)
        .context("Failed to get caller key pair")?;

    // Check if the account already exists
    let account_info = read_account_info_async(NODE1_ADDRESS.to_string(), caller_pubkey).await;
    if account_info.is_ok() {
        println!("  {} Account already exists", "ℹ".bold().blue());
        println!("  {} Account public key: {:?}", "ℹ".bold().blue(), hex::encode(caller_pubkey.serialize()));
        // Print account info
        println!("  {} Account info: {:?}", "ℹ".bold().blue(), account_info);

        return Ok(());
    }

    // Get account address
    let account_address = generate_account_address(caller_pubkey).await?;

    // Set up Bitcoin RPC client
    let wallet_manager = WalletManager::new(config)?;

    // Prompt user to send funds
    println!("{}", "Please send funds to the following address:".bold());
    println!("  {} Bitcoin address: {}", "→".bold().blue(), account_address.yellow());
    println!("  {} Minimum required: {} satoshis", "ℹ".bold().blue(), "3000".yellow());
    println!("  {} Waiting for funds...", "⏳".bold().blue());

    // Wait for funds (you may need to implement this function)
    // wait_for_funds(&wallet_manager.client, &account_address, config).await?;

    // sleep 5 seconds
    // tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    create_arch_account(&caller_keypair, &caller_pubkey, &account_address, &wallet_manager, config).await?;

    // Determine the program ID to transfer ownership to
    let program_id = if let Some(hex_program_id) = &args.program_id {
        if hex_program_id.is_empty() {
            println!("  {} No program ID provided. Using system program.", "ℹ".bold().blue());
            Pubkey::system_program()
        } else {
            // Convert hex string to bytes
            let program_id_bytes = hex::decode(hex_program_id)
                .context("Failed to decode program ID from hex")?;

            // Create Pubkey from bytes
            Pubkey::from_slice(&program_id_bytes)
        }
    } else {
        println!("  {} No program ID provided. Using system program.", "ℹ".bold().blue());
        Pubkey::system_program()
    };

    println!("  {} Program ID: {}", "ℹ".bold().blue(), program_id.to_string().yellow());

    // Transfer ownership to the program
    transfer_account_ownership(&caller_keypair, &caller_pubkey, &program_id).await?;

    println!("{}", "Account created and ownership transferred successfully!".bold().green());
    Ok(())
}

async fn generate_account_address(caller_pubkey: Pubkey) -> Result<String> {    // Get program account address from network
    let account_address = get_account_address_async(caller_pubkey)
        .await
        .context("Failed to get account address")?;
    // println!("  {} Account address: {}", "ℹ".bold().blue(), account_address.yellow());

    Ok(account_address)
}


async fn wait_for_funds(client: &Client, address: &str, config: &Config) -> Result<()> {
    // Check if wallet_manager.client is connected
    let connected = client.get_blockchain_info()?;
    println!("  {} Connected: {:?}", "ℹ".bold().blue(), connected);

    let tx_info = fund_address(client, address, config).await?;

    if let Some(info) = tx_info {
        println!("  {} Transaction confirmed with {} confirmations", "✓".bold().green(), info.info.confirmations.to_string().yellow());
    }

    Ok(())
}

async fn create_arch_account(caller_keypair: &Keypair, caller_pubkey: &Pubkey, account_address: &str, wallet_manager: &WalletManager, config: &Config) -> Result<()> {

    let tx_info = fund_address(&wallet_manager.client, &account_address, config).await?;

    // Output the bitcoin transaction info
    println!("  {} Transaction info: {:?}", "ℹ".bold().blue(), tx_info);

    if let Some(info) = tx_info {

        let (txid, _) = sign_and_send_instruction_async(
            SystemInstruction::new_create_account_instruction(
                hex::decode(&info.info.txid.to_string()).unwrap().try_into().unwrap(),
                0,
                *caller_pubkey
            ),
            vec![*caller_keypair],
        )
        .await.expect("signing and sending a transaction should not fail");

        println!("  {} Account created with transaction ID: {}", "✓".bold().green(), txid.yellow());
        Ok(())
    } else {
        println!("  {} Warning: No transaction info available for deployment", "⚠".bold().yellow());
        // You might want to implement an alternative deployment method for non-REGTEST networks
        Ok(())
    }
}

async fn transfer_account_ownership(caller_keypair: &Keypair, account_pubkey: &Pubkey, program_pubkey: &Pubkey) -> Result<()> {

    let mut instruction_data = vec![3]; // Transfer instruction
    instruction_data.extend(program_pubkey.serialize());

    println!("  {} Account public key: {:?}", "ℹ".bold().blue(), hex::encode(account_pubkey.serialize()));

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
    .await.expect("signing and sending a transaction should not fail");

    Ok(())
}