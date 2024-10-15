use anyhow::anyhow;
use anyhow::{Context, Result};
use arch_program::account::AccountMeta;
use arch_program::instruction::Instruction;
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction::SystemInstruction;
use bitcoin::key::UntweakedKeypair;
use bitcoin::{Address, XOnlyPublicKey};
use bitcoin::Amount;
use bitcoin::Network;
use bitcoincore_rpc::jsonrpc::serde_json;
use bitcoincore_rpc::{Client, RpcApi};
use clap::{Args, Parser, Subcommand};
use colored::*;
use common::constants::*;
use common::helper::*;
use common::runtime_transaction::RuntimeTransaction;
use config::{Config, Environment, File};
use dialoguer::theme::ColorfulTheme;
use rand::rngs::OsRng;
use secp256k1::Keypair;
use secp256k1::{Secp256k1, SecretKey};
use dialoguer::{Select, Input, Confirm};
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
use arch_program::message::Message;
use std::str::FromStr;
use std::time::Duration;
use dirs::home_dir;
use toml_edit::{Document, Item, value};
use indicatif::{ProgressBar, ProgressStyle};
use common::helper::*;

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

    /// Clean the project
    #[clap(long_about = "Removes the src/app directory, cleaning the project structure.")]
    Clean,
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Create a new project
    #[clap(long_about = "Creates a new project with a specified name.")]
    Create(CreateProjectArgs),
}

#[derive(Subcommand)]
pub enum IndexerCommands {
    /// Start the indexer
    #[clap(long_about = "Starts the arch-indexer using Docker Compose.")]
    Start,

    /// Stop the indexer
    #[clap(long_about = "Stops the arch-indexer using Docker Compose.")]
    Stop,

    /// Clean the indexer
    #[clap(long_about = "Removes the indexer data and configuration files.")]
    Clean,
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
pub struct CreateProjectArgs {
    /// Name of the project
    #[clap(short, long)]
    pub name: Option<String>,
}

#[derive(Args, Clone, Debug)]
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

    /// Folder within the project directory to deploy
    #[clap(
        long,
        help = "Specifies the folder within the project directory to deploy"
    )]
    folder: Option<String>,
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

    // Ensure default config exists
    ensure_default_config()?;

    // Store the current directory
    let cli_dir = std::env::current_dir()?;

    // Get the default project directory based on the OS
    let default_dir = get_default_project_dir();

    // Ask the user where they want to create the project
    let mut project_dir = prompt_for_project_dir(&default_dir)?;

    // Ensure the project directory is empty or create it
    loop {
        if !project_dir.exists() {
            println!(
                "  {} Directory does not exist. Do you want to create it? (Y/n)",
                "ℹ".bold().blue()
            );
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() != "n" {
                fs::create_dir_all(&project_dir)?;
                println!("  {} Directory created successfully", "✓".bold().green());
                break;
            } else {
                project_dir = prompt_for_project_dir(&default_dir)?;
            }
        } else if is_directory_empty(&project_dir)? {
            break;
        } else {
            println!(
                "  {} Directory is not empty. Do you want to use this existing project folder? (y/N)",
                "⚠".bold().yellow()
            );
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() == "y" {
                println!("  {} Using existing project folder", "✓".bold().green());
                break;
            } else {
                project_dir = prompt_for_project_dir(&default_dir)?;
            }
        }
    }

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

        // Update config.toml with project directory
        update_config_with_project_dir(&dest_path, &project_dir)?;
    } else {
        println!(
            "  {} Warning: config.default.toml not found",
            "⚠".bold().yellow()
        );
    }

    // Create the 'demo' folder within the project directory if it doesn't exist
    let demo_dir = project_dir.join("demo");
    if !demo_dir.exists() {
        // Create the 'demo' folder within the project directory
        fs::create_dir_all(&demo_dir)?;
        println!("  {} Created demo directory at {:?}", "✓".bold().green(), demo_dir);

        // Change to the CLI project directory
        std::env::set_current_dir(&cli_dir)?;

        // Copy everything from ./src to the demo folder
        println!("{}", "Copying project files...".bold().blue());
        println!(" Current directory: {:?}", std::env::current_dir()?);
        let src_dir = cli_dir.join("src");
        if src_dir.exists() {
            let exclude_files = &["lib.rs", "main.rs"];
            copy_dir_excluding(&src_dir, &demo_dir, exclude_files)?;
            println!("  {} Copied project files to demo directory", "✓".bold().green());
        } else {
            println!("  {} Warning: ./src directory not found", "⚠".bold().yellow());
        }

        // Copy the whole program folder to the demo folder
        println!("{}", "Copying program folder...".bold().blue());
        let program_dir = cli_dir.join("program");
        if program_dir.exists() {
            let program_dir_in_demo = demo_dir.join("program");
            fs::create_dir_all(&program_dir_in_demo)?;
            copy_dir_all(&program_dir, &program_dir_in_demo)?;
            println!("  {} Copied program folder to demo directory", "✓".bold().green());
        } else {
            println!("  {} Warning: ./program directory not found", "⚠".bold().yellow());
        }

        // Change to the demo directory
        std::env::set_current_dir(&demo_dir)?;

        // Build the program
        println!("{}", "Building Arch Network program...".bold().blue());
        let build_result = ShellCommand::new("cargo")
            .current_dir("program")
            .arg("build-sbf")
            .output();

        match build_result {
            Ok(output) if output.status.success() => {
                println!("  {} Arch Network program built successfully", "✓".bold().green());
            }
            Ok(output) => {
                println!(
                    "  {} Warning: Failed to build Arch Network program: {}",
                    "⚠".bold().yellow(),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                println!(
                    "  {} Warning: Failed to build Arch Network program: {}",
                    "⚠".bold().yellow(),
                    e
                );
            }
        }
    }

    println!(
        "  {} New Arch Network app initialized successfully!",
        "✓".bold().green()
    );
    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn copy_dir_excluding(src: impl AsRef<Path>, dst: impl AsRef<Path>, exclude: &[&str]) -> Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let filename = entry.file_name();

        // Skip excluded files
        if exclude.contains(&filename.to_str().unwrap_or("")) {
            continue;
        }

        if ty.is_dir() {
            copy_dir_excluding(entry.path(), dst.as_ref().join(entry.file_name()), exclude)?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn update_config_with_project_dir(config_path: &Path, project_dir: &Path) -> Result<()> {
    let config_content = fs::read_to_string(config_path)?;
    let mut doc = config_content.parse::<Document>()?;

    // Add a new [project] section if it doesn't exist
    if doc.get("project").is_none() {
        doc["project"] = toml_edit::table();
    }

    // Update the directory in the [project] section
    doc["project"]["directory"] = value(project_dir.to_str().unwrap());

    // Write the updated config back to the file
    fs::write(config_path, doc.to_string())?;

    println!(
        "  {} Updated configuration with project directory",
        "✓".bold().green()
    );

    Ok(())
}

fn is_directory_empty(path: &Path) -> Result<bool> {
    Ok(fs::read_dir(path)?.next().is_none())
}

fn get_default_project_dir() -> PathBuf {
    let mut path = home_dir().unwrap_or_else(|| PathBuf::from("."));
    if cfg!(windows) {
        path.push("Projects");
    } else if cfg!(target_os = "macos") {
        path.push("Documents");
    }
    path.push("ArchNetwork");
    path
}

fn prompt_for_project_dir(default_dir: &Path) -> Result<PathBuf> {
    println!("Where would you like to create your Arch Network project?");
    println!("Default: {}", default_dir.display());
    print!("Project directory (press Enter for default): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(default_dir.to_path_buf())
    } else {
        Ok(PathBuf::from(shellexpand::tilde(input).into_owned()))
    }
}

fn create_project_dir(project_dir: &Path) -> Result<()> {
    if !project_dir.exists() {
        println!(
            "  {} Directory does not exist. Creating it now...",
            "ℹ".bold().blue()
        );
        match fs::create_dir_all(project_dir) {
            Ok(_) => println!(
                "  {} Directory created successfully",
                "✓".bold().green()
            ),
            Err(e) => {
                return Err(anyhow!(
                    "Failed to create directory '{}': {}",
                    project_dir.display(),
                    e
                ))
            }
        }
    }
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

    // Get the project directory from the config
    let project_dir = PathBuf::from(config.get_string("project.directory")
        .context("Failed to get project directory from config")?);

    // Determine the deploy folder
    let deploy_folder = if let Some(folder) = &args.folder {
        project_dir.join(folder)
    } else {
        // List all folders in the project directory
        let folders = fs::read_dir(&project_dir)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    if e.file_type().ok()?.is_dir() {
                        Some(e.file_name().to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<String>>();

        if folders.is_empty() {
            return Err(anyhow!("No folders found in the project directory"));
        }

        println!("Available folders to deploy:");
        for (i, folder) in folders.iter().enumerate() {
            println!("  {}. {}", i + 1, folder);
        }

        let selected_folder = loop {
            let mut input = String::new();
            print!("Enter the number of the folder you want to deploy (or 'q' to quit): ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut input)?;

            let input = input.trim();
            if input.eq_ignore_ascii_case("q") {
                return Ok(());
            }

            if let Ok(choice) = input.parse::<usize>() {
                if choice > 0 && choice <= folders.len() {
                    break folders[choice - 1].clone();
                }
            }
            println!("Invalid selection. Please try again.");
        };

        project_dir.join(selected_folder)
    };

    println!("Deploying from folder: {:?}", deploy_folder);

    // Create a new DeployArgs with the updated folder
    let updated_args = DeployArgs {
        folder: Some(deploy_folder.to_str().unwrap().to_string()),
        ..args.clone()
    };

    // Build the program
    build_program(&updated_args)?;

    // Ensure the keys directory exists and load/generate the program keypair
    let (program_keypair, program_pubkey) = prepare_program_keys()?;

    // Display the program public key
    display_program_id(&program_pubkey);

    // Set up Bitcoin RPC client and handle funding
    let wallet_manager = WalletManager::new(config)?;
    ensure_wallet_balance(&wallet_manager.client).await?;

    // Get account address and fund it
    let account_address = get_account_address(program_pubkey);
    let tx_info = fund_address(&wallet_manager.client, &account_address, config).await?;

    // Deploy the program
    deploy_program_with_tx_info(&program_keypair, &program_pubkey, tx_info, deploy_folder.to_str().map(String::from)).await?;

    wallet_manager.close_wallet()?;

    println!(
        "{}",
        "Your app has been deployed successfully!".bold().green()
    );
    display_program_id(&program_pubkey);

    Ok(())
}
pub async fn server_stop() -> Result<()> {    println!("{}", "Stopping development server...".bold().yellow());

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

pub async fn server_clean() -> Result<()> {
    println!("{}", "Cleaning up the project...".bold().yellow());
    let config = load_config()?;
    let arch_data_dir = get_arch_data_dir(&config)?;
    let config_dir = get_config_dir()?;
    let keys_file = config_dir.join("keys.json");
    let config_file = config_dir.join("config.toml");

    // Ask user if they want to clean the indexer
    let clean_indexer = dialoguer::Confirm::new()
        .with_prompt("Do you want to clean the indexer? This will remove all indexer containers and data.")
        .default(false)
        .interact()?;

    if clean_indexer {
        println!("  {} Cleaning indexer...", "→".bold().blue());
        indexer_clean(&config).await?;
    } else {
        println!("  {} Indexer will be preserved", "ℹ".bold().blue());
    }

    // Ask user if they want to delete the keys.json file
    let delete_keys = dialoguer::Confirm::new()
        .with_prompt("Do you want to delete the keys.json file? This action cannot be undone.")
        .default(false)
        .interact()?;

    // Only ask about config.toml if indexer was cleaned
    let delete_config = if clean_indexer {
        dialoguer::Confirm::new()
            .with_prompt("Do you want to delete the config.toml file? This action cannot be undone.")
            .default(false)
            .interact()?
    } else {
        println!("  {} config.toml will be preserved as indexer was not cleaned", "ℹ".bold().blue());
        false
    };

    if arch_data_dir.exists() {
        fs::remove_dir_all(&arch_data_dir)?;
        println!("  {} Removed arch-data directory", "✓".bold().green());
    }

    if keys_file.exists() {
        if delete_keys {
            fs::remove_file(&keys_file)?;
            println!("  {} Removed keys.json file", "✓".bold().green());
        } else {
            println!("  {} Preserved keys.json file", "ℹ".bold().blue());
        }
    } else {
        println!("  {} No keys.json file found", "ℹ".bold().blue());
    }

    if config_file.exists() {
        if clean_indexer && delete_config {
            fs::remove_file(&config_file)?;
            println!("  {} Removed config.toml file", "✓".bold().green());
        } else {
            println!("  {} Preserved config.toml file", "ℹ".bold().blue());
        }
    } else {
        println!("  {} No config.toml file found", "ℹ".bold().blue());
    }

    // Stop and remove Docker containers for Bitcoin
    let bitcoin_compose_file = config.get_string("bitcoin.docker_compose_file").unwrap_or_default();
    if !bitcoin_compose_file.is_empty() {
        let status = Command::new("docker-compose")
            .args(["-f", &bitcoin_compose_file, "down", "--volumes"])
            .env("BITCOIN_RPC_USER", "")
            .env("ORD_PORT", "")
            .env("ELECTRS_REST_API_PORT", "")
            .env("ELECTRS_ELECTRUM_PORT", "")
            .env("BTC_RPC_EXPLORER_PORT", "")
            .status()
            .context("Failed to stop Bitcoin containers")?;

        if status.success() {
            println!("  {} Stopped and removed Bitcoin containers", "✓".bold().green());
        } else {
            println!("  {} Failed to stop Bitcoin containers", "✗".bold().red());
        }
    }

    // Stop and remove Docker containers for Arch
    let arch_compose_file = config.get_string("arch.docker_compose_file").unwrap_or_default();
    if !arch_compose_file.is_empty() {
        let status = Command::new("docker-compose")
            .args(["-f", &arch_compose_file, "down", "--volumes"])
            .env("BITCOIN_RPC_USER", "")
            .env("ORD_PORT", "")
            .env("ELECTRS_REST_API_PORT", "")
            .env("ELECTRS_ELECTRUM_PORT", "")
            .env("BTC_RPC_EXPLORER_PORT", "")
            .status()
            .context("Failed to stop Arch containers")?;

        if status.success() {
            println!("  {} Stopped and removed Arch containers", "✓".bold().green());
        } else {
            println!("  {} Failed to stop Arch containers", "✗".bold().red());
        }
    }

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

    // tokio::time::sleep(Duration::from_secs(25)).await;

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

async fn get_connected_peer_count(client: &reqwest::Client, rpc_endpoint: &str) -> Result<usize> {
    let rpc_request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "get_connected_peer_count",
        "params": [],
        "id": 1
    });

    let response = client
        .post(rpc_endpoint)
        .json(&rpc_request)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send RPC request: {:?}", e))?;

    if response.status().is_success() {
        let result: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse JSON response")?;

        result["result"]
            .as_u64()
            .map(|count| count as usize)
            .ok_or_else(|| anyhow!("Invalid peer count response"))
    } else {
        Err(anyhow!("Failed to get connected peer count"))
    }
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
            "  {} Warning: {} not found.",
            "⚠".bold().yellow(),
            config_path.display().to_string().yellow()
        );        
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
            .unwrap_or_else(|| "program".to_string()),
    );
    path.push("Cargo.toml");
    path
}

fn build_program(args: &DeployArgs) -> Result<()> {
    println!("  ℹ Building program...");

    // Print out the path to the Cargo.toml file
    println!("  ℹ Cargo.toml found at: {}", args.folder.clone().unwrap());

    // Change to the program directory
    let program_dir = args.folder.clone().unwrap();
    std::env::set_current_dir(&program_dir)
        .context("Failed to change to program directory")?;

    // Print out the current working directory
    println!("  ℹ Current working directory: {}", std::env::current_dir().unwrap().display());

    let output = std::process::Command::new("cargo")
        .args(["build-sbf", "--manifest-path", "app/program/Cargo.toml"])
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
}fn _get_program_key_path(args: &DeployArgs, config: &Config) -> Result<String> {
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
    deploy_folder: Option<String>,
) -> Result<()> {
    if let Some(info) = tx_info {
        deploy_program(
            program_keypair,
            program_pubkey,
            &info.info.txid.to_string(),
            0,
            deploy_folder
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

pub fn prepare_program_keys() -> Result<(secp256k1::Keypair, Pubkey)> {
    let config_dir = get_config_dir()?;
    let keys_file = config_dir.join("keys.json");

    if keys_file.exists() {
        let keys = load_keys(&keys_file)?;
        if !keys.as_object().map_or(true, |obj| obj.is_empty()) {
            return select_existing_key(&keys);
        }
    }

    create_new_key(&keys_file)
}

fn load_keys(keys_file: &PathBuf) -> Result<Value> {
    let keys_content = fs::read_to_string(keys_file)?;
    Ok(serde_json::from_str(&keys_content)?)
}

fn select_existing_key(keys: &Value) -> Result<(secp256k1::Keypair, Pubkey)> {
    let account_names: Vec<String> = keys.as_object().unwrap().keys().cloned().collect();
    let selection = Select::new()
        .with_prompt("Select a key to use as the program key")
        .items(&account_names)
        .default(0)
        .interact()?;

    let selected_account = &keys[&account_names[selection]];
    let secret_key = selected_account["secret_key"].as_str().unwrap();
    with_secret_key(secret_key)
}

fn create_new_key(keys_file: &PathBuf) -> Result<(secp256k1::Keypair, Pubkey)> {
    println!("No existing keys found or keys.json is empty.");
    if Confirm::new().with_prompt("Do you want to create a new key?").interact()? {
        let name = Input::<String>::new()
            .with_prompt("Enter a name for the new key")
            .interact_text()?;

        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let keypair = secp256k1::Keypair::from_secret_key(&secp, &secret_key);
        let pubkey = Pubkey::from_slice(&public_key.serialize()[1..33]); // Use only the 32-byte compressed public key

        save_keypair_to_json(keys_file, &keypair, &pubkey, &name)?;

        println!("New key created and saved as '{}'", name);
        Ok((keypair, pubkey))
    } else {
        Err(anyhow!("No key selected or created"))
    }
}

fn with_secret_key(secret_key_hex: &str) -> Result<(secp256k1::Keypair, Pubkey)> {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_str(secret_key_hex)?;
    let keypair = secp256k1::Keypair::from_secret_key(&secp, &secret_key);
    let public_key = keypair.public_key();
    let pubkey = Pubkey::from_slice(&public_key.serialize()[1..33]); // Use only the 32-byte compressed public key
    Ok((keypair, pubkey))
}

fn save_keypair_to_json(file_path: &PathBuf, keypair: &Keypair, pubkey: &Pubkey, name: &str) -> Result<()> {
    let mut keys: Value = if file_path.exists() {
        serde_json::from_str(&fs::read_to_string(file_path)?)?
    } else {
        json!({})
    };

    let account_info = json!({
        "public_key": hex::encode(pubkey.serialize()),
        "secret_key": hex::encode(keypair.secret_key().secret_bytes()),
    });

    keys[name] = account_info;

    fs::write(file_path, serde_json::to_string_pretty(&keys)?)?;
    Ok(())
}

fn generate_new_keypair() -> Result<(secp256k1::Keypair, Pubkey)> {
    let secp = Secp256k1::new();
    let (secret_key, _) = secp.generate_keypair(&mut OsRng);
    let keypair = secp256k1::Keypair::from_secret_key(&secp, &secret_key);
    let pubkey = Pubkey::from_slice(&keypair.public_key().serialize());
    Ok((keypair, pubkey))
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
    deploy_folder: Option<String>,
) -> Result<()> {
    // Create a new account for the program
    create_program_account(program_keypair, program_pubkey, txid, vout);

    // Deploy the program transactions
    deploy_program_txs_with_folder(program_keypair, program_pubkey, deploy_folder);

    // Make program executable
    make_program_executable(program_keypair, program_pubkey);

    Ok(())
}

fn make_program_executable(program_keypair: &Keypair, program_pubkey: &Pubkey) -> Result<()> {
    println!("    Making program executable...");
    let (txid, _) = sign_and_send_instruction(
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
    )?;
    println!("    Transaction sent: {}", txid.clone());
    get_processed_transaction(&NODE1_ADDRESS.to_string(), txid.clone())?;
    println!("    Program made executable successfully");
    Ok(())
}

pub fn deploy_program_txs(program_keypair: UntweakedKeypair, elf_path: &str) {
    let program_pubkey =
        Pubkey::from_slice(&XOnlyPublicKey::from_keypair(&program_keypair).0.serialize());

    let elf = fs::read(elf_path).expect("elf path should be available");

    //println!("Program size is : {} Bytes", elf.len());

    let txs = elf
        .chunks(extend_bytes_max_len())
        .enumerate()
        .map(|(i, chunk)| {
            let mut bytes = vec![];

            let offset: u32 = (i * extend_bytes_max_len()) as u32;
            let len: u32 = chunk.len() as u32;

            bytes.extend(offset.to_le_bytes());
            bytes.extend(len.to_le_bytes());
            bytes.extend(chunk);

            let message = Message {
                signers: vec![program_pubkey],
                instructions: vec![SystemInstruction::new_extend_bytes_instruction(
                    bytes,
                    program_pubkey,
                )],
            };

            let digest_slice =
                hex::decode(message.hash()).expect("hashed message should be decodable");

            RuntimeTransaction {
                version: 0,
                signatures: vec![common::signature::Signature(
                    sign_message_bip322(&program_keypair, &digest_slice).to_vec(),
                )],
                message,
            }
        })
        .collect::<Vec<RuntimeTransaction>>();

    /*println!(
        "Program deployment split into {} Chunks, sending {} runtime transactions",
        txs.len(),
        txs.len()
    );
     */
    let txids = process_result(post_data(NODE1_ADDRESS, "send_transactions", txs))
        .expect("send_transaction should not fail")
        .as_array()
        .expect("cannot convert result to array")
        .iter()
        .map(|r| {
            r.as_str()
                .expect("cannot convert object to string")
                .to_string()
        })
        .collect::<Vec<String>>();

    let pb = ProgressBar::new(txids.len() as u64);

    pb.set_style(ProgressStyle::default_bar()
        .progress_chars("#>-")
        .template("{spinner:.green}[{elapsed_precise:.blue}] {msg:.blue} [{bar:100.green/blue}] {pos}/{len} ({eta})").unwrap());

    pb.set_message("Successfully Processed Deployment Transactions :");

    for txid in txids {
        let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
            .expect("get processed transaction should not fail");
        pb.inc(1);
        pb.set_message("Successfully Processed Deployment Transactions :");
    }

    pb.finish();

    // for tx_batch in txs.chunks(12) {
    //     let mut txids = vec![];
    //     for tx in tx_batch {
    //         let txid = process_result(post_data(NODE1_ADDRESS, "send_transaction", tx))
    //             .expect("send_transaction should not fail")
    //             .as_str()
    //             .expect("cannot convert result to string")
    //             .to_string();

    //         println!("sent tx {:?}", txid);
    //         txids.push(txid);
    //     };

    //     for txid in txids {
    //         let processed_tx = get_processed_transaction(NODE1_ADDRESS, txid.clone())
    //             .expect("get processed transaction should not fail");

    //         println!("{:?}", read_account_info(NODE1_ADDRESS, program_pubkey.clone()));
    //     }
    // }
}

async fn deploy_program_txs_with_folder(program_keypair: &Keypair, _program_pubkey: &Pubkey, deploy_folder: Option<String>) -> Result<()> {
    println!("    Deploying program transactions...");

    let so_folder = deploy_folder
        .ok_or_else(|| anyhow!("No deploy folder specified"))?
        .to_string();
    let so_folder = format!("{}/app/program/target/sbf-solana-solana/release", so_folder);

    // Scan the deploy_folder for the .so file in the folder and set so_file to that
    let so_file = {
        let mut so_file = None;
        for file in fs::read_dir(&so_folder)? {
            let path = file?.path();
            if path.is_file() && path.extension().unwrap_or_default() == "so" {
                so_file = path.to_str().map(|s| s.to_string());
                break;
            }
        }
        so_file.ok_or_else(|| anyhow!("No .so file found in the specified folder"))?
    };

    deploy_program_txs(
        *program_keypair,
        &so_file,
    );
    println!("    Program transactions deployed successfully");
    Ok(())
}

async fn create_program_account(    program_keypair: &Keypair,
    program_pubkey: &Pubkey,
    txid: &str,
    vout: u32,
) -> Result<()> {
    println!("    Creating program account...");
    let (txid, _) = sign_and_send_instruction(
        SystemInstruction::new_create_account_instruction(
            hex::decode(txid).unwrap().try_into().unwrap(),
            vout,
            *program_pubkey,
        ),
        vec![*program_keypair],
    )?;
    get_processed_transaction(&NODE1_ADDRESS.to_string(), txid.clone());
    println!("    Program account created successfully");
    Ok(())
}

pub async fn demo_start(config: &Config) -> Result<()> {
    println!("{}", "Starting the demo application...".bold().green());

    set_env_vars(config)?;

    // Get the project directory from the config
    let project_dir = config.get_string("project.directory")
        .context("Failed to get project directory from config")?;

    // Change to the demo directory
    let demo_dir = PathBuf::from(project_dir).join("demo");
    std::env::set_current_dir(&demo_dir)
        .context("Failed to change to demo directory")?;

    let output = ShellCommand::new("docker-compose")
        .arg("-f")
        .arg("app/demo-docker-compose.yml")
        .arg("up")
        .arg("--build")
        .arg("-d")
        .output()
        .context("Failed to start the demo application using Docker Compose")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to start the demo application: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!(
        "{}",
        "Demo application started successfully!".bold().green()
    );
    Ok(())
}

pub async fn demo_stop(config: &Config) -> Result<()> {
    println!("{}", "Stopping the demo application...".bold().green());

    set_env_vars(config)?;

    // Get the project directory from the config
    let project_dir = config.get_string("project.directory")
        .context("Failed to get project directory from config")?;

    // Change to the demo directory
    let demo_dir = PathBuf::from(project_dir).join("demo");
    std::env::set_current_dir(&demo_dir)
        .context("Failed to change to demo directory")?;

    let output = ShellCommand::new("docker-compose")
        .arg("-f")
        .arg("app/demo-docker-compose.yml")
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
    let keys_dir = get_config_dir()?;
    let keys_file = keys_dir.join("keys.json");

    // Check if an account with the same name already exists
    if key_name_exists(&keys_file, &args.name)? {
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

    // Save the account information to keys.json
    save_keypair_to_json(&keys_file, &caller_keypair, &caller_pubkey, &args.name)?;

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
    let keys_dir = get_config_dir()?;
    let keys_file = keys_dir.join("keys.json");

    if !keys_file.exists() {
        println!("  {} No accounts found", "ℹ".bold().blue());
        return Ok(());
    }

    let keys = load_keys(&keys_file)?;

    println!("{}", "Stored accounts:".bold().green());
    for (name, account_info) in keys.as_object().unwrap() {
        println!(
            "  {} Account: {}",
            "→".bold().blue(),
            name.yellow()
        );
        println!(
            "    Public Key: {}",
            account_info["public_key"].as_str().unwrap()
        );
    }

    Ok(())
}

fn key_name_exists(keys_file: &PathBuf, name: &str) -> Result<bool> {
    if !keys_file.exists() {
        return Ok(false);
    }

    let keys = load_keys(keys_file)?;

    Ok(keys.as_object().unwrap().contains_key(name))
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
    let account_address = get_account_address(caller_pubkey);
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
        let (txid, _) = sign_and_send_instruction(
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

    let (_txid, _) = sign_and_send_instruction(
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
        .arg("--build")
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

// Remove the docker containers and associated volumes
pub async fn indexer_clean(config: &Config) -> Result<()> {
    println!("{}", "Cleaning up the arch-indexer...".bold().yellow());

    // Confirmation prompt
    let proceed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("This will remove all arch-indexer containers, data, and volumes. Are you sure you want to proceed?")
        .default(false)
        .interact()?;

    if !proceed {
        println!("  {} Operation cancelled.", "ℹ".bold().blue());
        return Ok(());
    }

    set_env_vars(config)?;

    // Stop and remove containers
    let output = Command::new("docker-compose")
        .arg("-f")
        .arg("arch-indexer/docker-compose.yml")
        .arg("down")
        .arg("-v")  // This will also remove named volumes declared in the "volumes" section
        .output()
        .context("Failed to stop and remove arch-indexer containers")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to stop and remove arch-indexer containers: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Remove the pgdata volume explicitly
    let output = Command::new("docker")
        .args(&["volume", "rm", "arch-indexer_pgdata"])
        .output()
        .context("Failed to remove arch-indexer_pgdata volume")?;

    if !output.status.success() {
        println!(
            "  {} Warning: Failed to remove arch-indexer_pgdata volume. It may not exist or may be in use.",
            "⚠".bold().yellow()
        );
    }

    println!(
        "{}",
        "Arch-indexer cleaned up successfully!".bold().green()
    );
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
        .arg("ghcr.io/arch-network/local_validator:latest")
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

pub async fn project_create(args: &CreateProjectArgs, config: &Config) -> Result<()> {
    let config_dir = get_config_dir()?;
    if !config_dir.exists() {
        println!("{}", "Config directory not found. It seems the 'init' command hasn't been run yet.".bold().yellow());
        let run_init = Confirm::new()
            .with_prompt("Do you want to run the 'init' command now?")
            .default(true)
            .interact()?;

        if run_init {
            init().await?;
        } else {
            return Err(anyhow!("Please run 'arch-cli init' before creating a project."));
        }
    } else {
        // Ensure the config file exists even if the directory does
        ensure_default_config()?;
    }

    println!("{}", "Creating a new project...".bold().green());

    // Get the project directory from the config or prompt the user
    let project_dir = match config.get_string("project.directory") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => {
            let default_dir = get_default_project_dir();
            prompt_for_project_dir(&default_dir)?
        }
    };

    // Ensure the project directory exists
    if !project_dir.exists() {
        fs::create_dir_all(&project_dir)
            .context(format!("Failed to create project directory: {:?}", project_dir))?;
        println!("  {} Created project directory at {:?}", "✓".bold().green(), project_dir);
    }

    // Update the config with the new project directory
    update_config_with_project_dir(&get_config_path()?, &project_dir)?;

    // Get project name, either from args or by asking the user
    let mut project_name = args.name.clone().unwrap_or_default();
    if project_name.is_empty() {
        project_name = Input::<String>::new()
            .with_prompt("Enter a name for your project")
            .interact()?;
    }

     // Create the new project folder
     let mut new_project_dir = project_dir.join(&project_name);

     // Check if the folder already exists and ask for a new name if it does
     while new_project_dir.exists() {
         println!("  {} A project with this name already exists.", "⚠".bold().yellow());
         let use_existing = Confirm::new()
             .with_prompt("Do you want to use the existing project?")
             .default(false)
             .interact()?;

         if use_existing {
             println!("  {} Using existing project directory.", "ℹ".bold().blue());
             break;
         } else {
             project_name = Input::<String>::new()
                 .with_prompt("Enter a new name for your project")
                 .interact()?;
             new_project_dir = project_dir.join(&project_name);
         }
     }

    // Create the project directory if it doesn't exist
    if !new_project_dir.exists() {
        fs::create_dir_all(&new_project_dir)
            .context(format!("Failed to create project directory: {:?}", new_project_dir))?;
        println!("  {} Created project directory at {:?}", "✓".bold().green(), new_project_dir);
    }

    // Get the CLI directory (where the template files are located)
    let cli_dir = std::env::current_dir()?;

    // Copy the dummy program folder to the new project directory
    let src_program_dir = cli_dir.join("src/app/program");
    let dest_program_dir = new_project_dir.join("app/program");
    copy_dir_all(&src_program_dir, &dest_program_dir)
        .context("Failed to copy program folder")?;

    println!("  {} Copied program template to {:?}", "✓".bold().green(), dest_program_dir);

    // Copy the program folder to the new project directory
    let src_program_dir = cli_dir.join("program");
    let dest_program_dir = new_project_dir.join("program");
    copy_dir_all(&src_program_dir, &dest_program_dir)
        .context("Failed to copy program folder")?;

    println!("  {} Copied program to {:?}", "✓".bold().green(), dest_program_dir);

    // Copy the src/common folder over to the proejct directory
    let src_common_dir = cli_dir.join("src/common");
    let dest_common_dir = new_project_dir.join("common");
    copy_dir_all(&src_common_dir, &dest_common_dir)
        .context("Failed to copy common folder")?;

    println!("  {} Copied common libraries to {:?}", "✓".bold().green(), dest_common_dir);

    // Create a basic README.md file
    let readme_content = format!("# {}\n\nThis is a new Arch Network project.", project_name);
    fs::write(new_project_dir.join("README.md"), readme_content)
        .context("Failed to create README.md")?;

    println!("  {} Created README.md", "✓".bold().green());

    println!("{}", "New project created successfully!".bold().green());
    println!("  {} Project location: {:?}", "ℹ".bold().blue(), new_project_dir);
    // println!("  {} To get started, navigate to the project directory and run 'arch-cli init'", "ℹ".bold().blue());

    Ok(())
}

fn ensure_default_config() -> Result<()> {
    let config_path = get_config_path()?;
    let config_dir = config_path.parent().unwrap();

    if !config_path.exists() {
        // Create the arch-data directory if it doesn't exist
        let arch_data_dir = config_dir.join("arch-data");
        fs::create_dir_all(&arch_data_dir)?;
        println!(
            "  {} Created arch-data directory at {:?}",
            "✓".bold().green(),
            arch_data_dir
        );

        // Copy config.default.toml to the config directory
        let default_config_path = Path::new("config.default.toml");
        if default_config_path.exists() {
            fs::copy(default_config_path, &config_path)
                .with_context(|| format!("Failed to copy default config to {:?}", config_path))?;
            println!(
                "  {} Copied default configuration to {:?}",
                "✓".bold().green(),
                config_path
            );
        } else {
            println!(
                "  {} Warning: config.default.toml not found",
                "⚠".bold().yellow()
            );
            return Err(anyhow!("Default configuration file not found"));
        }
    }

    Ok(())
}