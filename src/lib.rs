mod demo;
use demo::{setup_demo_environment, build_frontend, get_cloud_run_url};
use anyhow::anyhow;
use anyhow::{Context, Result};
use arch_program::account::AccountMeta;
use arch_program::instruction::Instruction;
use arch_program::message::Message;
use arch_program::pubkey::Pubkey;
use arch_program::system_instruction::SystemInstruction;
use rand::{distributions::Alphanumeric, Rng};
use bitcoin::key::UntweakedKeypair;
use bitcoin::Amount;
use bitcoin::Network;
use bitcoin::{Address, XOnlyPublicKey};
use bitcoincore_rpc::jsonrpc::serde_json;
use bitcoincore_rpc::{Client, RpcApi};
use bitcoincore_rpc::json::EstimateMode;
use clap::{Args, Parser, Subcommand};
use colored::*;
use common::constants::*;
use common::helper::*;
use common::helper::*;
use common::runtime_transaction::RuntimeTransaction;
use config::{Config, Environment, File};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input, Select};
use dirs::home_dir;
use bip322::{sign_message_bip322};
use indicatif::{ProgressBar, ProgressStyle};
use rand::rngs::OsRng;
use secp256k1::Keypair;
use secp256k1::{Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use webbrowser::open_browser;
use std::collections::HashMap;
use std::env;
use regex::Regex;
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
use tokio::task;
use toml_edit::{value, Document, Item, Array};
use include_dir::{include_dir, Dir};

use common::wallet_manager::*;

static PROJECT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates/demo");
static SAMPLE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates/sample");
static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");

#[derive(Deserialize)]
pub struct ServiceConfig {
    #[allow(dead_code)]
    docker_compose_file: String,
    services: Vec<String>,
}

#[derive(Deserialize)]
pub struct NetworkConfig {
    pub type_: String,
    pub bitcoin_rpc_endpoint: String,
    pub bitcoin_rpc_port: String,
    pub bitcoin_rpc_user: String,
    pub bitcoin_rpc_password: String,
    pub bitcoin_rpc_wallet: String,
    pub docker_compose_file: Option<String>,
    pub leader_rpc_endpoint: String,
    pub services: Option<Vec<String>>,
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

    /// Specify the network to use (development, development2, testnet, mainnet)
    #[clap(long, global = true, default_value = "development")]
    pub network: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new Arch Network app
    #[clap(long_about = "Creates the project structure and configuration for a new Arch Network application.")]
    Init,

    /// Manage the development server
    #[clap(subcommand)]
    Server(ServerCommands),

    /// Deploy your Arch Network app
    #[clap(long_about = "Builds and deploys your Arch Network application to the specified network.")]
    Deploy(DeployArgs),

    /// Manage projects
    #[clap(subcommand)]
    Project(ProjectCommands),

    /// Manage the Distributed Key Generation (DKG) process
    #[clap(subcommand)]
    Dkg(DkgCommands),

    /// Manage Bitcoin operations
    #[clap(subcommand)]
    Bitcoin(BitcoinCommands),

    /// Manage the demo application
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
    #[clap(long_about = "Starts the local development environment, including Bitcoin regtest network and Arch Network nodes.")]
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

    /// Clean the project
    #[clap(long_about = "Removes temporary files and resets the development environment.")]
    Clean,
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Create a new project
    #[clap(long_about = "Creates a new Arch Network project with the specified name.")]
    Create(CreateProjectArgs),

    /// Deploy a project
    #[clap(long_about = "Deploys the specified Arch Network project.")]
    Deploy,
}

#[derive(Subcommand)]
pub enum IndexerCommands {
    /// Start the indexer
    #[clap(long_about = "Starts the arch-indexer using Docker Compose.")]
    Start(IndexerStartArgs),

    /// Stop the indexer
    #[clap(long_about = "Stops the arch-indexer using Docker Compose.")]
    Stop(IndexerStartArgs),

    /// Clean the indexer
    #[clap(long_about = "Removes the indexer data and configuration files.")]
    Clean,
}

#[derive(Args)]
pub struct IndexerStartArgs {
    /// Deployment target (local or gcp)
    #[clap(
        long,
        default_value = "local",
        help = "Specifies where to deploy the indexer: local or gcp"
    )]
    target: String,

    /// GCP configuration (required for GCP deployment)
    #[clap(long, help = "GCP project ID")]
    gcp_project: Option<String>,

    #[clap(long, help = "GCP region")]
    gcp_region: Option<String>,

    #[clap(long, help = "GCP machine type")]
    gcp_machine_type: Option<String>,

    /// RPC URL for connecting to the Arch Network
    #[clap(long, help = "RPC URL for the Arch Network node")]
    rpc_url: Option<String>,
}

#[derive(Subcommand)]
pub enum ValidatorCommands {
    /// Start the validator
    #[clap(long_about = "Starts a local validator with specified network settings.")]
    Start(ValidatorStartArgs),

    /// Stop the validator
    #[clap(long_about = "Stops the local validator.")]
    Stop(ValidatorStartArgs),
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
    /// Start the demo application
    #[clap(long_about = "Starts the demo application.")]
    Start(DemoStartArgs),

    /// Stop the demo application
    #[clap(long_about = "Stops the demo application.")]
    Stop,
}

#[derive(Subcommand)]
pub enum AccountCommands {
    /// Create a new account
    #[clap(long_about = "Creates a new account without assigning program ownership")]
    Create(CreateAccountArgs),

    /// List all accounts
    #[clap(long_about = "Lists all accounts stored in the accounts file")]
    List,

    /// Delete an account
    #[clap(long_about = "Deletes an account from the accounts file")]
    Delete(DeleteAccountArgs),

    /// Assign program ownership to an account
    #[clap(long_about = "Transfers ownership of an account to a program")]
    AssignOwnership(AssignOwnershipArgs),

    /// Update account data
    #[clap(long_about = "Updates the account data from a file")]
    Update(UpdateAccountArgs),
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
    /// Program ID to transfer ownership to (optional)
    #[clap(long, help = "Specifies the program ID to transfer ownership to")]
    program_id: Option<String>,

    /// Custom name for the account
    #[clap(long, help = "Specifies a custom name for the account")]
    name: String,

    /// RPC URL for connecting to the Arch Network
    #[clap(long, help = "RPC URL for the Arch Network node")]
    rpc_url: Option<String>,
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
    /// Path to the compiled ELF binary (optional)
    #[clap(
        long,
        help = "Path to the compiled ELF binary file. If not provided, will compile from source"
    )]
    elf_path: Option<String>,

    /// Directory containing the program source (optional if elf_path is provided)
    #[clap(
        long,
        help = "Directory containing your Arch Network program source"
    )]
    directory: Option<String>,

    /// Path to the program key file (optional)
    #[clap(
        long,
        help = "Path to a file containing hex-encoded private key for deployment"
    )]
    program_key: Option<String>,

    /// RPC URL for connecting to the Arch Network
    #[clap(long, help = "RPC URL for the Arch Network node")]
    rpc_url: Option<String>,
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
pub struct DemoStartArgs {
    /// Deployment target (local or gcp)
    #[clap(
        long,
        default_value = "local",
        help = "Specifies where to deploy the demo: local or gcp"
    )]
    target: String,

    /// GCP configuration (required for GCP deployment)
    #[clap(long, help = "GCP project ID")]
    gcp_project: Option<String>,

    #[clap(long, help = "GCP region")]
    gcp_region: Option<String>,

    /// RPC URL for connecting to the Arch Network
    #[clap(long, help = "RPC URL for the Arch Network node")]
    rpc_url: Option<String>,

    /// Skip cleanup of existing containers and network
    #[clap(long, help = "Skip stopping existing containers and removing network")]
    skip_cleanup: bool,
}

#[derive(Args)]
pub struct ValidatorStartArgs {
    /// Network to use (testnet or mainnet)
    #[clap(
        long,
        default_value = "development",
        help = "Specifies the network to use: development, development2, testnet, or mainnet"
    )]
    network: String,

    /// Deployment target (local or gcp)
    #[clap(
        long,
        default_value = "local",
        help = "Specifies where to deploy the validator: local or gcp"
    )]
    target: String,

    /// GCP configuration (required for GCP deployment)
    #[clap(long, help = "GCP project ID")]
    gcp_project: Option<String>,

    #[clap(long, help = "GCP region")]
    gcp_region: Option<String>,

    #[clap(long, help = "GCP machine type")]
    gcp_machine_type: Option<String>,
}

#[derive(Args)]
pub struct AssignOwnershipArgs {
    /// Account name or ID to assign ownership
    #[clap(help = "Name or ID of the account to assign ownership")]
    identifier: String,

    /// Program ID to transfer ownership to
    #[clap(long, help = "Program ID to transfer ownership to")]
    program_id: String,

    /// RPC URL for connecting to the Arch Network
    #[clap(long, help = "RPC URL for the Arch Network node")]
    rpc_url: Option<String>,
}

#[derive(Args)]
pub struct UpdateAccountArgs {
    /// Account name or ID to update
    #[clap(help = "Name or ID of the account to update")]
    identifier: String,

    /// Path to the data file
    #[clap(long, help = "Path to file containing the account data bytes")]
    data_file: PathBuf,

    /// RPC URL for connecting to the Arch Network
    #[clap(long, help = "RPC URL for the Arch Network node")]
    rpc_url: Option<String>,
}

pub async fn init() -> Result<()> {
    println!("{}", "Initializing new Arch Network app...".bold().green());

    // Check dependencies
    check_dependencies()?;

    // Ensure default config exists
    ensure_default_config()?;

    // Get the default project directory based on the OS
    let default_dir = get_default_project_dir();

    // Ask the user where they want to create the project
    let mut project_dir = prompt_for_project_dir(&default_dir)?;

    // Get the configuration file path
    let config_path = get_config_path()?;
    update_config_with_project_dir(&config_path, &project_dir)?;

    // Create the arch-data directory
    let config_dir = config_path.parent().unwrap();
    let arch_data_dir = config_dir.join("arch-data");
    fs::create_dir_all(&arch_data_dir)?;

    println!(
        "  {} Created arch-data directory at {:?}",
        "✓".bold().green(),
        arch_data_dir
    );

    let config = Config::builder()
        .add_source(File::with_name(config_path.to_str().unwrap()))
        .build()?;
    let (_, _projects_dir) = setup_base_structure(&config)?;

    // Create the 'demo' folder within the project directory if it doesn't exist
    let demo_dir = project_dir.join("projects/demo");
    if !demo_dir.exists() {
        // Create the 'demo' folder within the project directory
        fs::create_dir_all(&demo_dir)?;
        println!(
            "  {} Created demo directory at {:?}",
            "✓".bold().green(),
            demo_dir
        );

        // Extract project files from binary
        extract_recursive(&PROJECT_DIR, &demo_dir)?;

        // Rename the .env.example file to .env
        let env_example_file = PathBuf::from(&demo_dir).join("app/frontend/.env.example");
        if env_example_file.exists() {
            fs::rename(&env_example_file, PathBuf::from(&demo_dir).join("app/frontend/.env"))?;
        }

        // Change to the demo directory
        std::env::set_current_dir(&project_dir)?;

        // Build the program
        println!("{}", "Building Arch Network program...".bold().blue());
        let build_result = ShellCommand::new("cargo")
            .current_dir("program")
            .arg("build-sbf")
            .output();

        match build_result {
            Ok(output) if output.status.success() => {
                println!(
                    "  {} Arch Network program built successfully",
                    "✓".bold().green()
                );
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

fn extract_project_files(project_dir: &Dir, target_dir: &Path) -> Result<()> {
    for entry in project_dir.entries() {
        match entry {
            include_dir::DirEntry::File(file) => {
                let relative_path = file.path();
                let target_path = target_dir.join(relative_path);
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(target_path, file.contents())?;
            }
            include_dir::DirEntry::Dir(dir) => {
                let dir_name = dir.path().file_name().unwrap().to_str().unwrap();
                if dir_name == "app" {
                    // App directory should be at the top level
                    extract_dir_contents(dir, &target_dir.join(dir_name))?;
                } else {
                    // Other directories follow the original structure
                    let new_target_dir = target_dir.join(dir.path());
                    extract_dir_contents(dir, &new_target_dir)?;
                }
            }
        }
    }
    Ok(())
}

fn extract_dir_contents(dir: &Dir, target_path: &Path) -> Result<()> {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::File(file) => {
                let relative_path = file.path();
                let target_file = target_path.join(relative_path);
                if let Some(parent) = target_file.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(target_file, file.contents())?;
            }
            include_dir::DirEntry::Dir(subdir) => {
                let new_target = target_path.join(subdir.path());
                fs::create_dir_all(&new_target)?;
                extract_dir_contents(subdir, &new_target)?;
            }
        }
    }
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

fn copy_dir_excluding(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    exclude: &[&str],
) -> Result<()> {
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
    println!("⚠️  📝 Note: Please provide a full path (e.g., /home/user/projects or C:\\Users\\user\\Projects) 📝 ⚠️");
    print!("Project directory (press Enter for default): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(default_dir.to_path_buf())
    } else {
        // Expand ~ if present
        let expanded_path = shellexpand::tilde(input).into_owned();

        // Convert to PathBuf
        let path = PathBuf::from(&expanded_path);

        // If the path is not absolute, make it absolute by joining with current directory
        if !path.is_absolute() {
            println!("  🚨 {} Warning: Relative path detected! Converting to absolute path... 🚨", "⚠".bold().yellow());
            let current_dir = std::env::current_dir()?;
            let absolute_path = current_dir.join(path);
            println!("  💡 {} Using absolute path: {} 💡", "ℹ".bold().blue(), absolute_path.display());
            Ok(absolute_path)
        } else {
            Ok(path)
        }
    }
}

pub async fn create_project(args: &CreateProjectArgs, config: &Config) -> Result<()> {
    println!("{}", "Creating new project...".bold().green());

    // Set up base structure
    let (base_dir, projects_dir) = setup_base_structure(config)?;

    // Get project name
    let project_name = args.name.clone().unwrap_or_else(|| {
        Input::<String>::new()
            .with_prompt("Enter a name for your project")
            .interact()
            .unwrap()
    });

    // Create project directory within projects directory
    let project_dir = projects_dir.join(&project_name);
    if project_dir.exists() {
        return Err(anyhow!("Project {} already exists", project_name));
    }

    // Create the project directory and extract sample contents
    fs::create_dir_all(&project_dir)?;

    for entry in SAMPLE_DIR.entries() {
        match entry {
            include_dir::DirEntry::File(file) => {
                let target_path = project_dir.join(file.path().file_name().unwrap());
                println!("    {} Extracting file: {:?}", "📄".bold().blue(), target_path);
                fs::write(target_path, file.contents())?;
            }
            include_dir::DirEntry::Dir(dir) => {
                let dir_name = dir.path().file_name().unwrap();
                let target_path = project_dir.join(dir_name);
                println!("    {} Extracting directory: {:?}", "📁".bold().blue(), target_path);

                fs::create_dir_all(&target_path)?;
                for subentry in dir.entries() {
                    match subentry {
                        include_dir::DirEntry::File(file) => {
                            let file_path = target_path.join(file.path().file_name().unwrap());
                            println!("      {} Extracting file: {:?}", "📄".bold().blue(), file_path);
                            fs::write(file_path, file.contents())?;
                        }
                        include_dir::DirEntry::Dir(subdir) => {
                            let subdir_name = subdir.path().file_name().unwrap();
                            let subdir_path = target_path.join(subdir_name);
                            println!("      {} Extracting directory: {:?}", "📁".bold().blue(), subdir_path);

                            // Create subdirectory and copy its contents directly
                            fs::create_dir_all(&subdir_path)?;
                            for item in subdir.entries() {
                                match item {
                                    include_dir::DirEntry::File(file) => {
                                        let file_path = subdir_path.join(file.path().file_name().unwrap());
                                        fs::write(file_path, file.contents())?;
                                    }
                                    include_dir::DirEntry::Dir(d) => {
                                        let d_path = subdir_path.join(d.path().file_name().unwrap());
                                        fs::create_dir_all(&d_path)?;
                                        for f in d.entries() {
                                            if let include_dir::DirEntry::File(file) = f {
                                                fs::write(d_path.join(file.path().file_name().unwrap()), file.contents())?;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Create Vite app using npm
    println!("Creating Vite application...");
    let create_vite_output = std::process::Command::new("npm")
        .args(["create", "vite@latest", "frontend", "--", "--template", "react"])
        .current_dir(&project_dir.join("app"))
        .output()
        .context("Failed to create Vite application")?;

    if !create_vite_output.status.success() {
        return Err(anyhow!("Failed to create Vite application: {}",
            String::from_utf8_lossy(&create_vite_output.stderr)));
    }
    println!("  {} Created Vite application", "✓".bold().green());

    // Change to frontend directory and install base dependencies
    let frontend_dir = project_dir.join("app/frontend");
    let install_output = std::process::Command::new("npm")
        .arg("install")
        .current_dir(&frontend_dir)
        .output()
        .context("Failed to install base dependencies")?;

    if !install_output.status.success() {
        return Err(anyhow!("Failed to install base dependencies: {}",
            String::from_utf8_lossy(&install_output.stderr)));
    }
    println!("  {} Installed base dependencies", "✓".bold().green());

    // Install additional packages
    let additional_packages = ["sats-connect", "@saturnbtcio/arch-sdk"];
    let install_additional_output = std::process::Command::new("npm")
        .arg("install")
        .args(&additional_packages)
        .current_dir(&frontend_dir)
        .output()
        .context("Failed to install additional packages")?;

    if !install_additional_output.status.success() {
        return Err(anyhow!("Failed to install additional packages: {}",
            String::from_utf8_lossy(&install_additional_output.stderr)));
    }
    println!("  {} Installed additional packages", "✓".bold().green());

    println!("{}", "Project created successfully! 🎉".bold().green());
    println!("Project location: {:?}", project_dir);

    println!("\n{}", "Next steps:".bold().yellow());
    println!(
        "  1. Navigate to {} to find the Rust program template",
        project_dir.join("app").join("program").display().to_string().yellow()
    );
    println!("  2. Edit the source code to implement your program logic");
    println!("  3. When ready, run {} to compile and deploy your program to the network", "arch-cli deploy".cyan());
    println!("\n{}", "Need help? Check out our documentation at https://docs.arch.network".italic());

    Ok(())
}

fn extract_recursive(dir: &Dir, target_path: &Path) -> Result<()> {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::Dir(subdir) => {
                let target_dir = target_path.join(subdir.path().file_name().unwrap());
                fs::create_dir_all(&target_dir)?;
                extract_recursive(subdir, &target_dir)?;
            }
            include_dir::DirEntry::File(file) => {
                let target_file = target_path.join(file.path().file_name().unwrap());
                fs::write(target_file, file.contents())?;
            }
        }
    }
    Ok(())
}

pub fn setup_base_structure(config: &Config) -> Result<(PathBuf, PathBuf)> {
    // Get base project directory from config
    let base_dir = PathBuf::from(config.get_string("project.directory")?);

    // Create base directory if it doesn't exist
    if !base_dir.exists() {
        fs::create_dir_all(&base_dir)?;
        println!("  {} Created base directory at {:?}", "✓".bold().green(), base_dir);
    }

    // Create shared libraries at base directory level
    println!("  {} Setting up shared libraries...", "ℹ".bold().blue());
    for lib in &["bip322", "common", "program"] {
        let source_dir = TEMPLATES_DIR.get_dir(lib)
            .ok_or_else(|| anyhow!("Template directory '{}' not found", lib))?;
        let lib_dir = base_dir.join(lib);

        // Create the library directory
        fs::create_dir_all(&lib_dir)?;

        // Extract contents directly to the library directory, maintaining original structure
        for entry in source_dir.entries() {
            match entry {
                include_dir::DirEntry::Dir(subdir) => {
                    let target_dir = lib_dir.join(subdir.path().file_name().unwrap());
                    fs::create_dir_all(&target_dir)?;
                    extract_recursive(subdir, &target_dir)?;
                }
                include_dir::DirEntry::File(file) => {
                    let target_file = lib_dir.join(file.path().file_name().unwrap());
                    fs::write(target_file, file.contents())?;
                }
            }
        }

        println!("  {} Created shared library {} at {:?}", "✓".bold().green(), lib, lib_dir);
    }

    // Create projects directory
    let projects_dir = base_dir.join("projects");
    fs::create_dir_all(&projects_dir)?;
    println!("  {} Created projects directory at {:?}", "✓".bold().green(), projects_dir);

    Ok((base_dir, projects_dir))
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
    println!("{}", "Starting the server...".bold().green());

    let arch_data_dir = get_arch_data_dir(config)?;
    env::set_var("ARCH_DATA_DIR", arch_data_dir.to_str().unwrap());

    let selected_network = config.get_string("selected_network")
        .unwrap_or_else(|_| "development".to_string());

    set_env_vars(config, &selected_network)?;

    let docker_compose_file = config.get_string(&format!("networks.{}.docker_compose_file", selected_network))?;
    let docker_compose_file = format!("{}/{}", config.get_string("config_dir")?, docker_compose_file);

    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();

    println!("  {} Starting services...", "→".bold().blue());

    let output = Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(["-f", &docker_compose_file, "up", "-d"])
        .env("ARCH_DATA_DIR", arch_data_dir.to_str().unwrap())
        .status()?;

    if !output.success() {
        return Err(anyhow!("Failed to start services"));
    }

    println!(
        "  {} Development server started successfully.",
        "✓".bold().green()
    );

    Ok(())
}

pub async fn server_stop(config: &Config) -> Result<()> {
    println!("{}", "Stopping the development server...".bold().green());

    let selected_network = config.get_string("selected_network")
        .unwrap_or_else(|_| "development".to_string());

    set_env_vars(config, &selected_network)?;

    let docker_compose_file = config.get_string(&format!("networks.{}.docker_compose_file", selected_network))?;
    let docker_compose_file = format!("{}/{}", config.get_string("config_dir")?, docker_compose_file);

    let (docker_compose_cmd, docker_compose_args) = get_docker_compose_command();

    println!("  {} Stopping services...", "→".bold().blue());

    let output = Command::new(docker_compose_cmd)
        .args(docker_compose_args)
        .args(["-f", &docker_compose_file, "down", "-v", "--remove-orphans"])
        .status()?;

    if !output.success() {
        return Err(anyhow!("Failed to stop services"));
    }

    println!(
        "  {} Development server stopped successfully.",
        "✓".bold().green()
    );

    Ok(())
}

pub async fn deploy(args: &DeployArgs, config: &Config) -> Result<()> {
    println!("{}", "Deploying program...".bold().green());

    // Find the program binary or compile from source
    let program_path = if let Some(dir) = &args.directory {
        PathBuf::from(dir)
    } else {
        // Get project directory from config
        let project_dir = PathBuf::from(config.get_string("project.directory")?);
        let projects_dir = project_dir.join("projects");

        // Get list of projects
        let projects: Vec<_> = fs::read_dir(&projects_dir)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    if path.is_dir() && path.join("app/program").exists() {
                        Some(path.file_name().unwrap().to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
            })
            .collect();

        if projects.is_empty() {
            return Err(anyhow!("No deployable projects found. Make sure your projects have an 'app/program' folder."));
        }

        // Ask user to select a project
        let selection = Select::new()
            .with_prompt("Select a project to deploy")
            .items(&projects)
            .interact()?;

        let selected_project = &projects[selection];
        projects_dir.join(selected_project).join("app/program")
    };

    // Handle program key selection
    let secp = Secp256k1::new();
    let keys_file = get_config_dir()?.join("keys.json");

    let program_keypair = if let Some(key_path) = &args.program_key {
        // Load from provided key file
        let key_path = PathBuf::from(key_path);
        if !key_path.exists() {
            return Err(anyhow!("Program key file not found at: {}", key_path.display()));
        }
        let hex_key = fs::read_to_string(&key_path)?.trim().to_string();
        let key_bytes = hex::decode(&hex_key)
            .map_err(|e| anyhow!("Invalid hex-encoded private key: {}", e))?;

        UntweakedKeypair::from_seckey_slice(&secp, &key_bytes)
            .map_err(|e| anyhow!("Invalid private key: {}", e))?
    } else {
        // Show key selection menu
        let mut keys: Value = if keys_file.exists() {
            serde_json::from_str(&fs::read_to_string(&keys_file)?)?
        } else {
            json!({})
        };

        let (selected_keypair, _) = select_existing_key(&mut keys)?;
        selected_keypair
    };

    let program_pubkey = Pubkey::from_slice(
        &XOnlyPublicKey::from_keypair(&program_keypair).0.serialize()
    );

    println!("Program ID: {}", program_pubkey);

    // Set up Bitcoin RPC client and handle funding
    let wallet_manager = WalletManager::new(config)?;
    ensure_wallet_balance(&wallet_manager.client).await?;

    // Deploy the program
    let rpc_url = get_rpc_url_with_fallback(args.rpc_url.clone(), config).unwrap();
    println!("Using RPC URL: {}", rpc_url);

    // Get the program binary path
    let elf_path = if program_path.is_file() {
        program_path
    } else {
        // Compile from source
        println!("  {} Compiling program...", "→".bold().blue());
        let status = Command::new("cargo")
            .current_dir(&program_path)
            .arg("build-sbf")
            .status()
            .context("Failed to run cargo build-sbf")?;

        if !status.success() {
            return Err(anyhow!("Failed to compile program"));
        }

        // Find the compiled binary
        let target_dir = program_path.join("target/deploy");
        fs::read_dir(&target_dir)?
            .filter_map(Result::ok)
            .find(|entry| entry.path().extension().map_or(false, |ext| ext == "so"))
            .ok_or_else(|| anyhow!("No .so file found in target/deploy directory"))?
            .path()
    };

    // Deploy the program
    let rpc_url = get_rpc_url_with_fallback(args.rpc_url.clone(), config).unwrap();

    // Deploy the program
    deploy_program_from_path(
        &elf_path,
        config,
        Some((program_keypair.clone(), program_pubkey)),
        rpc_url.clone(),
    ).await?;

    // Make the program executable
    make_program_executable(&program_keypair, &program_pubkey, rpc_url).await?;

    println!("{}", "Program deployed successfully!".bold().green());
    Ok(())
}

pub fn create_unique_key_name(keys_file: &Path) -> Result<String> {
    let mut counter = 1;
    let base_name = "program_key";
    
    let existing_keys: Value = if keys_file.exists() {
        serde_json::from_str(&fs::read_to_string(keys_file)?)?
    } else {
        json!({})
    };

    while existing_keys.get(&format!("{}{}", base_name, counter)).is_some() {
        counter += 1;
    }

    Ok(format!("{}{}", base_name, counter))
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
        None,                    // comment
        None,                    // comment_to
        Some(false),            // subtract_fee
        Some(true),             // replaceable (enable RBF)
        Some(1),                // confirmation_target (blocks)
        Some(EstimateMode::Economical) // estimate_mode
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
            "��".bold().blue(),
            network_type.yellow()
        );
    }

    Ok(())
}

fn fetch_service_logs(service_name: &str, services: &[String]) -> Result<()> {
    println!(
        "  {} Fetching logs for {}...",
        "→".bold().blue(),
        service_name.yellow()
    );

    for container in services {
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
    println!("{}", format!("Fetching logs for {}...", service).bold().blue());

    let network_type = config.get_string("selected_network").unwrap_or_else(|_| "development".to_string());

    if network_type != "development" && network_type != "development2" {
        println!("  {} Logs are not available for non-development networks", "ℹ".bold().blue());
        return Ok(());
    }

    let services_to_fetch = match service {
        "all" => vec!["bitcoin", "arch"],
        s if s == "bitcoin" || s == "arch" => vec![s],
        _ => return Err(anyhow!("Invalid service specified")),
    };

    for &s in &services_to_fetch {
        let config_key = if s == "bitcoin" {
            format!("networks.{}.services", network_type)
        } else {
            "arch.services".to_string()
        };

        if let Ok(services) = config.get_array(&config_key) {
            let service_names: Vec<String> = services.iter()
                .filter_map(|v| Some(v.to_string()))
                .collect();

            if !service_names.is_empty() {
                fetch_service_logs(&format!("{} services", s), &service_names)?;
            } else {
                println!("  {} No services defined for {}", "ℹ".bold().blue(), s);
            }
        } else {
            println!("  {} Failed to get services for {}", "⚠".bold().yellow(), s);
        }
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

pub async fn server_clean(config: &Config) -> Result<()> {
    println!("{}", "Cleaning up the project...".bold().yellow());
    let arch_data_dir = get_arch_data_dir(&config)?;
    let config_dir = get_config_dir()?;
    let keys_file = config_dir.join("keys.json");
    let config_file = config_dir.join("config.toml");

    // Ask user if they want to clean the indexer
    let clean_indexer = dialoguer::Confirm::new()
        .with_prompt(
            "Do you want to clean the indexer? This will remove all indexer containers and data.",
        )
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
            .with_prompt(
                "Do you want to delete the config.toml file? This action cannot be undone.",
            )
            .default(false)
            .interact()?
    } else {
        println!(
            "  {} config.toml will be preserved as indexer was not cleaned",
            "ℹ".bold().blue()
        );
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
    let bitcoin_compose_file = config
        .get_string("bitcoin.docker_compose_file")
        .unwrap_or_default();
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
            println!(
                "  {} Stopped and removed Bitcoin containers",
                "✓".bold().green()
            );
        } else {
            println!("  {} Failed to stop Bitcoin containers", "✗".bold().red());
        }
    }

    // Stop and remove Docker containers for Arch
    let arch_compose_file = config
        .get_string("arch.docker_compose_file")
        .unwrap_or_default();
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
            println!(
                "  {} Stopped and removed Arch containers",
                "✓".bold().green()
            );
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
        .get_string("leader_rpc_endpoint")
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

pub fn load_config(network: &str) -> Result<Config> {
    let config_path = get_config_path()?;
    let config_dir = config_path.parent().unwrap().to_str().unwrap().to_string();
    println!("Loading config for network: {}", network);
    
    let mut builder = Config::builder();
    
    // Load default configuration first
    let default_config = include_str!("../templates/config.default.toml");
    builder = builder.add_source(config::File::from_str(
        default_config,
        config::FileFormat::Toml
    ));

    // Check if the user config file exists
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

    // Add environment variables and set config_dir
    builder = builder
        .add_source(Environment::with_prefix("ARCH_CLI").separator("_"))
        .set_override("config_dir", config_dir.clone())?;

    // Build the initial configuration
    let initial_config = builder.build()?;

    // Check if e2e network configuration exists
    let e2e_config: Option<Value> = initial_config.get("networks.e2e").ok();

    if e2e_config.is_none() {
        println!(
            "  {} e2e network configuration not found, creating from default...",
            "ℹ".bold().blue()
        );

        // If config file doesn't exist, create the directory and file
        if !config_path.exists() {
            fs::create_dir_all(&config_dir)?;
            fs::write(&config_path, default_config)?;
            println!(
                "  {} Created new config file at {}",
                "✓".bold().green(),
                config_path.display().to_string().yellow()
            );
        } else {
            // Read existing config
            let mut config_content = fs::read_to_string(&config_path)?;

            // Parse default config to get e2e network section
            let default_toml: Value = toml::from_str(default_config)?;
            let e2e_network = default_toml
                .get("networks")
                .and_then(|n| n.get("e2e"))
                .ok_or_else(|| anyhow!("e2e network configuration not found in default config"))?;

            // Add e2e network section to existing config
            if !config_content.contains("[networks.e2e]") {
                config_content.push_str("\n\n[networks.e2e]\n");
                config_content.push_str(&toml::to_string(e2e_network)?);

                fs::write(&config_path, config_content)?;
                println!(
                    "  {} Added e2e network configuration to {}",
                    "✓".bold().green(),
                    config_path.display().to_string().yellow()
                );
            }
        }

        // Reload the configuration
        builder = Config::builder()
            .add_source(config::File::from_str(
                &fs::read_to_string(&config_path)?,
                config::FileFormat::Toml
            ))
            .add_source(Environment::with_prefix("ARCH_CLI").separator("_"))
            .set_override("config_dir", config_dir)?;
    }

    // Try to get the network-specific configuration
    let network_config: Option<Value> = initial_config.get(&format!("networks.{}", network)).ok();

    if let Some(network_config) = network_config {
        // Merge the network-specific configuration
        builder = Config::builder()
            .add_source(config::File::from_str(
                &serde_json::to_string(&network_config)?,
                config::FileFormat::Json,
            ))
            .add_source(initial_config);

        println!(
            "  {} Loaded network-specific configuration for {}",
            "✓".bold().green(),
            network.yellow()
        );
    } else {
        println!(
            "  {} No specific configuration found for network {}",
            "ℹ".bold().blue(),
            network.yellow()
        );
        builder = Config::builder().add_source(initial_config);
    }

    // Add the network key to the final configuration
    builder = builder.set_override("selected_network", network.to_string())?;

    // Set the bitcoin.network based on the selected network
    let bitcoin_network = match network {
        "mainnet" => "bitcoin",
        "testnet" => "testnet",
        "development" => "regtest",
        "e2e" => "regtest",
        _ => "regtest", // Default to regtest if unknown
    };
    builder = builder.set_override("bitcoin.network", bitcoin_network)?;

    builder = builder.set_override("leader_rpc_endpoint", "http://localhost:9002")?;

    // Build the final configuration
    let final_config = builder
        .build()
        .context("Failed to build configuration")?;

    Ok(final_config)
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
fn set_env_vars(config: &Config, network: &str) -> Result<()> {
    let network_config: std::collections::HashMap<String, config::Value> = config
        .get_table(&format!("networks.{}", network))
        .with_context(|| format!("Failed to get configuration for network '{}'", network))?;

    let vars = [
        ("BITCOIN_RPC_ENDPOINT", "bitcoin_rpc_endpoint"),
        ("BITCOIN_RPC_PORT", "bitcoin_rpc_port"),
        ("BITCOIN_RPC_USER", "bitcoin_rpc_user"),
        ("BITCOIN_RPC_PASSWORD", "bitcoin_rpc_password"),
        ("BITCOIN_RPC_WALLET", "bitcoin_rpc_wallet"),
        ("LEADER_RPC_ENDPOINT", "leader_rpc_endpoint"),
    ];

    for (env_var, config_key) in vars.iter() {
        if let Some(value) = network_config.get(*config_key) {
            if let Ok(str_value) = value.clone().into_string() {
                env::set_var(env_var, str_value);
            }
        }
    }

    // Set other environment variables that are not network-specific
    let other_vars = [
        ("ELECTRS_REST_API_PORT", "electrs.rest_api_port"),
        ("ELECTRS_ELECTRUM_PORT", "electrs.electrum_port"),
        ("BTC_RPC_EXPLORER_PORT", "btc_rpc_explorer.port"),
        ("DEMO_FRONTEND_PORT", "demo.frontend_port"),
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

    for (env_var, config_key) in other_vars.iter() {
        if let Ok(value) = config.get_string(config_key) {
            env::set_var(env_var, value);
        }
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

fn get_program_path(args: &DeployArgs, config: &Config) -> Result<PathBuf> {
    match &args.elf_path {
        Some(path) => Ok(PathBuf::from(path)),
        None => {
            let (_, projects_dir) = setup_base_structure(config)?;
            let projects: Vec<_> = fs::read_dir(&projects_dir)?
                .filter_map(Result::ok)
                .filter(|e| e.path().join("program").exists())
                .collect();

            if projects.is_empty() {
                return Err(anyhow!("No deployable projects found"));
            }

            let selections: Vec<_> = projects.iter()
                .map(|p| p.file_name().to_string_lossy().into_owned())
                .collect();

            let selection = Select::new()
                .with_prompt("Select a project to deploy")
                .items(&selections)
                .interact()?;

            Ok(projects[selection].path().join("program"))
        }
    }
}

fn _get_program_key_path(args: &DeployArgs, config: &Config) -> Result<String> {
    Ok(args.program_key.clone().unwrap_or_else(|| config.get_string("program.key_path").unwrap_or_default()))
}

async fn deploy_program_with_tx_info(
    program_keypair: &bitcoin::secp256k1::Keypair,
    program_pubkey: &arch_program::pubkey::Pubkey,
    tx_info: Option<bitcoincore_rpc::json::GetTransactionResult>,
    deploy_folder: Option<String>,
    config: &Config,
    rpc_url: String,
) -> Result<()> {
    if let Some(info) = tx_info {
        deploy_program(
            program_keypair,
            program_pubkey,
            &info.info.txid.to_string(),
            0,
            deploy_folder.map(|folder| format!("{}/app/program", folder)),
            config,
            rpc_url,
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
        let mut keys = load_keys(&keys_file)?;
        if !keys.as_object().map_or(true, |obj| obj.is_empty()) {
            return select_existing_key(&mut keys);
        }
    }

    create_new_key(&keys_file)
}
fn load_keys(keys_file: &PathBuf) -> Result<Value> {
    let keys_content = fs::read_to_string(keys_file)?;
    Ok(serde_json::from_str(&keys_content)?)
}

fn select_existing_key(keys: &mut Value) -> Result<(secp256k1::Keypair, Pubkey)> {
    let mut account_names: Vec<String> = keys.as_object().unwrap().keys().cloned().collect();
    account_names.push("Create a new key".to_string());

    let selection = Select::new()
        .with_prompt("Select a key to use as the program key")
        .items(&account_names)
        .default(0)
        .interact()?;

    if selection == account_names.len() - 1 {
        // User chose to create a new key
        let new_key_name = Input::<String>::new()
            .with_prompt("Enter a name for the new key")
            .interact_text()?;

        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        let keypair = secp256k1::Keypair::from_secret_key(&secp, &secret_key);
        let pubkey = Pubkey::from_slice(&public_key.serialize()[1..33]);

        // Save the new key to the keys Value
        let new_key_value = json!({
            "public_key": hex::encode(pubkey.serialize()),
            "secret_key": hex::encode(secret_key.secret_bytes()),
        });
        keys[&new_key_name] = new_key_value;

        // Save the updated keys to the file
        let keys_file = get_config_dir()?.join("keys.json");
        fs::write(&keys_file, serde_json::to_string_pretty(keys)?)?;

        println!("  {} Created and saved new key '{}'", "✓".bold().green(), new_key_name);

        Ok((keypair, pubkey))
    } else {
        // User selected an existing key
        let selected_account = &keys[&account_names[selection]];
        let secret_key = selected_account["secret_key"].as_str().unwrap();
        with_secret_key(secret_key)
    }
}

fn create_new_key(keys_file: &PathBuf) -> Result<(secp256k1::Keypair, Pubkey)> {
    println!("No existing keys found or keys.json is empty.");
    if Confirm::new()
        .with_prompt("Do you want to create a new key?")
        .interact()?
    {
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

fn save_keypair_to_json(
    file_path: &PathBuf,
    keypair: &Keypair,
    pubkey: &Pubkey,
    name: &str,
) -> Result<()> {
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

    // Handle the public key format consistently
    let serialized_pubkey = keypair.public_key().serialize();
    let pubkey = if serialized_pubkey.len() == 33 {
        // Compressed key format
        Pubkey::from_slice(&serialized_pubkey[1..33])
    } else if serialized_pubkey.len() == 65 {
        // Uncompressed key format
        Pubkey::from_slice(&serialized_pubkey[1..33])
    } else {
        return Err(anyhow!("Invalid public key length: {}", serialized_pubkey.len()));
    };

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
        let checked_address = new_address.require_network(arch_program::bitcoin::Network::Regtest)?;
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

        println!("Network: {}", bitcoin_network);

    let address = Address::from_str(account_address).context("Invalid account address")?;
    let checked_address = address
        .require_network(bitcoin_network)
        .context("Account address does not match the configured Bitcoin network")?;

    if bitcoin_network == Network::Regtest || bitcoin_network == Network::Testnet {
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

        println!("Sending funds to address: {}", checked_address.to_string());

        let tx = rpc.send_to_address(
            &checked_address,
            Amount::from_sat(5000),
            None,                           // comment
            None,                           // comment_to
            Some(false),                    // subtract_fee_from_amount
            None,                           // replaceable (RBF)
            Some(1),                        // conf_target (1 block for high priority)
            Some(bitcoincore_rpc::json::EstimateMode::Economical), // estimate_mode
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

        // Create a progress bar for waiting
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} {msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈"),
        );

        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(3600); // 60 minutes timeout

        // Wait for transaction confirmation
        loop {
            if start_time.elapsed() > timeout {
                pb.finish_with_message("❌ Transaction confirmation timed out after 60 minutes");
                return Err(anyhow!("Transaction confirmation timed out"));
            }

            match rpc.get_transaction(&tx, None) {
                Ok(info) if info.info.confirmations > 0 => {
                    pb.finish_with_message(format!(
                        "✓ Transaction confirmed with {} confirmations",
                        info.info.confirmations.to_string().yellow()
                    ));
                    return Ok(Some(info));
                }
                Ok(_) => {
                    let elapsed = start_time.elapsed().as_secs();
                    pb.set_message(format!(
                        "Waiting for confirmation... ({:02}:{:02})",
                        elapsed / 60,
                        elapsed % 60
                    ));
                }
                Err(e) => {
                    pb.set_message(format!(
                        "⚠ Error checking transaction: {}. Retrying...",
                        e.to_string()
                    ));
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await; // Check every 5 seconds instead of 1
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

        // Implement balance checking for non-REGTEST networks
        loop {
            let balance = rpc.get_balance(None, None)?;
            if balance > Amount::from_sat(5000) {
                println!("  {} Funds received", "✓".bold().green());
                return Ok(None);
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    Ok(None)
}

pub fn get_rpc_url_with_fallback(rpc_url: Option<String>, config: &Config) -> Result<String> {
    // Check if rpc_url is Some and not empty
    match rpc_url {
        Some(url) if !url.trim().is_empty() => Ok(url),
        _ => {
            // If rpc_url is None or empty, try getting from config
            match config.get_string("leader_rpc_endpoint") {
                Ok(url) if !url.trim().is_empty() => Ok(url),
                _ => Ok(common::constants::NODE1_ADDRESS.to_string())
            }
        }
    }
}

async fn deploy_program(
    program_keypair: &Keypair,
    program_pubkey: &Pubkey,
    txid: &str,
    vout: u32,
    deploy_folder: Option<String>,
    config: &Config,
    rpc_url: String,
) -> Result<()> {
    // Create a new account for the program
    create_program_account(program_keypair, program_pubkey, txid, vout, rpc_url.clone()).await?;

    // Deploy the program transactions
    deploy_program_txs_with_folder(program_keypair, program_pubkey, deploy_folder, config, rpc_url.clone()).await?;

    // Make program executable
    tokio::task::block_in_place(move || {
        make_program_executable(program_keypair, program_pubkey, rpc_url)
    }).await?;

    Ok(())
}

fn build_program_from_path(program_dir: &PathBuf) -> Result<()> {
    println!("  ℹ Building program...");

    // Change to the program directory
    std::env::set_current_dir(program_dir).context("Failed to change to program directory")?;

    let output = Command::new("cargo")
        .args(["build-sbf", "--manifest-path", "Cargo.toml"])
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

pub async fn deploy_program_from_path(
    program_dir: &PathBuf,
    config: &Config,
    keypair: Option<(Keypair, Pubkey)>,
    rpc_url: String,
) -> Result<()> {
    println!("  ℹ Deploying program...");

    // Get or prepare program keys
    let (program_keypair, program_pubkey) = keypair.ok_or_else(|| anyhow!("No keypair provided"))?;

    // Build the program if it's a directory
    if program_dir.is_dir() {
        build_program_from_path(program_dir)?;
    }

    // Find the .so file
    let so_file_path = find_program_so_file(program_dir)?;

    // Deploy the program
    deploy_program_txs(
        &so_file_path,
        &program_keypair,
        &program_pubkey,
        config,
        rpc_url,
    ).await?;

    println!("  ✓ Program deployed successfully");
    display_program_id(&program_pubkey);
    Ok(())
}

async fn make_program_executable(
    program_keypair: &Keypair,
    program_pubkey: &Pubkey,
    rpc_url: String,
) -> Result<()> {
    println!("    Making program executable...");

    let instruction = Instruction {
        program_id: Pubkey::system_program(),
        accounts: vec![AccountMeta {
            pubkey: *program_pubkey,
            is_signer: true,
            is_writable: true,
        }],
        data: vec![2],
    };

    let keypair = program_keypair.clone();
    let rpc_url_clone = rpc_url.clone();
    let (txid, _) = tokio::task::spawn_blocking(move || {
        sign_and_send_instruction(instruction, vec![keypair], rpc_url_clone)
    }).await??;

    println!("    Transaction sent: {}", txid);

    let rpc_url_clone = rpc_url.clone();
    tokio::task::spawn_blocking(move || {
        get_processed_transaction(&rpc_url_clone, txid.clone())
    }).await??;

    println!("    Program made executable successfully");
    Ok(())
}

async fn deploy_program_txs(
    so_file_path: &PathBuf,
    program_keypair: &Keypair,
    program_pubkey: &Pubkey,
    config: &Config,
    rpc_url: String,
) -> Result<()> {
    println!("  ℹ Deploying program from: {:?}", so_file_path);

    // Read the .so file
    let elf = fs::read(so_file_path)
        .with_context(|| format!("Failed to read .so file at {:?}", so_file_path))?;

    let network = config.get_string("bitcoin.network")
        .unwrap_or_else(|_| "regtest".to_string());
    let bitcoin_network =
        Network::from_str(&network).context("Invalid Bitcoin network specified in config")?;

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
                signers: vec![*program_pubkey],
                instructions: vec![SystemInstruction::new_extend_bytes_instruction(
                    bytes,
                    *program_pubkey,
                )],
            };

            let digest_slice = message.hash();

            RuntimeTransaction {
                version: 0,
                signatures: vec![common::signature::Signature(
                    sign_message_bip322(&program_keypair, &digest_slice, bitcoin_network).to_vec(),
                )],
                message,
            }
        })
        .collect::<Vec<RuntimeTransaction>>();

    let url = rpc_url.clone();
    let url_clone = url.clone();

    let txids: Vec<String> = {
        let txs_clone = txs.clone();
        let response = task::spawn_blocking(move || {
            post_data(&url_clone, "send_transactions", txs_clone)
        }).await?;

        process_result(response)
            .map_err(|e| anyhow!("Failed to process result: {}", e))?
            .as_array()
            .ok_or_else(|| anyhow!("Result is not an array"))?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    };

    let pb = ProgressBar::new(txids.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .progress_chars("#>-")
        .template("{spinner:.green}[{elapsed_precise:.blue}] {msg:.blue} [{bar:100.green/blue}] {pos}/{len} ({eta})").unwrap());
    pb.set_message("Processing Deployment Transactions:");

    for txid in txids {
        let url_clone = url.clone();
        let txid_clone = txid.clone();
        task::spawn_blocking(move || {
            get_processed_transaction(&url_clone, txid_clone)
        }).await??;
        pb.inc(1);
    }

    pb.finish();
    Ok(())
}

async fn deploy_program_txs_with_folder(
    program_keypair: &Keypair,
    program_pubkey: &Pubkey,
    deploy_folder: Option<String>,
    config: &Config,
    rpc_url: String,
) -> Result<()> {
    println!("    Deploying program transactions...");

    let program_dir = deploy_folder
        .ok_or_else(|| anyhow!("No deploy folder specified"))?;

    println!("  ℹ Program directory: {}", program_dir);

    // Pass the program directory directly without modifying the path
    let program_dir = PathBuf::from(program_dir);

    if let Err(e) = deploy_program_txs(
        &program_dir,
        program_keypair,
        program_pubkey,
        config,
        rpc_url,
    ).await {
        println!("Failed to deploy program transactions: {}", e);
        return Err(e);
    }
    println!("    Program transactions deployed successfully");
    Ok(())
}

async fn create_program_account(
    program_keypair: &Keypair,
    program_pubkey: &Pubkey,
    txid: &str,
    vout: u32,
    rpc_url: String,
) -> Result<()> {
    println!("    Creating program account...");

    let program_keypair_clone = program_keypair.clone();
    let program_pubkey_clone = *program_pubkey;
    let txid_clone = txid.to_string();
    let url = rpc_url.clone();

    let (txid, _) = tokio::task::spawn_blocking(move || {
        sign_and_send_instruction(
            SystemInstruction::new_create_account_instruction(
                hex::decode(txid_clone).unwrap().try_into().unwrap(),
                vout,
                program_pubkey_clone,
            ),
            vec![program_keypair_clone],
            url,
        )
    }).await??;

    Ok(())
}

pub async fn demo_start(args: &DemoStartArgs, config: &Config) -> Result<()> {
    println!("{}", "Starting the demo application...".bold().green());

    match args.target.as_str() {
        "local" => start_local_demo(args, config).await,
        "gcp" => start_gcp_demo(args, config).await,
        _ => Err(anyhow!("Invalid deployment target. Use 'local' or 'gcp'"))
    }
}

pub async fn start_local_demo(args: &DemoStartArgs, config: &Config) -> Result<()> {
    println!("{}", "Starting the demo application...".bold().green());

    // Get the selected network from the config
    let selected_network = config.get_string("selected_network")
        .unwrap_or_else(|_| "development".to_string());

    // Set environment variables for the selected network
    set_env_vars(config, &selected_network)?;

    // Get the project directory from the config
    let project_dir = config
        .get_string("project.directory")
        .context("Failed to get project directory from config")?;

    // Define the demo directory in the project
    let demo_dir = PathBuf::from(&project_dir).join("projects/demo");

    // Check if the demo directory exists, if not, copy it from the CLI directory
    if !demo_dir.exists() {
        println!(
            "  {} Demo directory not found. Creating it...",
            "ℹ".bold().blue()
        );

        // Create the 'demo' folder within the project directory
        fs::create_dir_all(&demo_dir)?;
        println!(
            "  {} Created demo directory at {:?}",
            "✓".bold().green(),
            demo_dir
        );

        // Extract demo files from binary
        extract_recursive(&PROJECT_DIR, &demo_dir)?;

        // Rename the .env.example file to .env
        let env_example_file = PathBuf::from(&demo_dir).join("app/frontend/.env.example");
        if env_example_file.exists() {
            fs::rename(&env_example_file, PathBuf::from(&demo_dir).join("app/frontend/.env"))?;
        }

        println!(
            "  {} Extracted demo template to {:?}",
            "✓".bold().green(),
            demo_dir
        );
    }

    // Change to the demo directory
    std::env::set_current_dir(&demo_dir).context("Failed to change to demo directory")?;

    let env_file = PathBuf::from(&demo_dir).join("app/frontend/.env");
    println!("  {} Attempting to read .env file from: {:?}", "ℹ".bold().blue(), env_file);

    // Check if the file exists
    if !env_file.exists() {
        println!("  {} .env file not found", "✗".bold().red());

        // Check if .env.example exists
        let env_example_file = PathBuf::from(&demo_dir).join("app/frontend/.env.example");
        if env_example_file.exists() {
            println!("  {} Found .env.example file, attempting to copy it", "→".bold().blue());
            fs::copy(&env_example_file, &env_file)
                .with_context(|| format!("Failed to copy .env.example to .env. Source: {:?}, Destination: {:?}", env_example_file, env_file))?;
            println!("  {} Successfully created .env file from .env.example", "✓".bold().green());
        } else {
            println!("  {} Neither .env nor .env.example files found in frontend directory", "✗".bold().red());
            return Err(anyhow!("Missing required .env files in {:?}", env_file.parent().unwrap_or_else(|| Path::new(""))));
        }
    }

    // Read the VITE_PROGRAM_PUBKEY from the specific .env file
    let env_content = fs::read_to_string(&env_file)
        .with_context(|| {
            let metadata = fs::metadata(&env_file);
            let permissions = metadata.as_ref().map(|m| m.permissions());
            format!(
                "Failed to read .env file at {:?}. File exists: {}, Metadata: {:?}, Permissions: {:?}",
                env_file,
                env_file.exists(),
                metadata,
                permissions
            )
        })?;

    // Verify the content is not empty
    if env_content.trim().is_empty() {
        println!("  {} Warning: .env file is empty", "⚠".bold().yellow());
    }

    let mut program_pubkey = env_content
        .lines()
        .find_map(|line| line.strip_prefix("VITE_PROGRAM_PUBKEY="))
        .unwrap_or("")
        .to_string();

    println!("  {} Read program pubkey from .env: '{}'", "ℹ".bold().blue(),
        if program_pubkey.is_empty() { "not found" } else { &program_pubkey });

    // If program_pubkey is empty then create a new account for the program
    let keys_file = get_config_dir()?.join("keys.json");

    let graffiti_key_name: String;

    if program_pubkey.is_empty() {
        // Check if there is a key in keys.json with the name graffiti and add a postfix until it is unique
        graffiti_key_name = {
            let mut name = String::from("graffiti");
            let mut counter = 1;
            while key_name_exists(&keys_file, &name)? {
                name = format!("graffiti_{}", counter);
                counter += 1;
            }
            name
        };

        println!("Creating account with name: {}", graffiti_key_name);

        // Call create_account with the graffiti_key_name
        create_account(&CreateAccountArgs {
            name: graffiti_key_name.clone(),
            program_id: None,
            rpc_url: Some(args.rpc_url.clone().unwrap_or_default()),
        }, config).await?;

        // Set the program_pubkey to the pubkey of the graffiti account
        program_pubkey = get_pubkey_from_name(&graffiti_key_name, &keys_file)?;

        // Write the program_pubkey into the app/frontend/.env file
        let env_file = PathBuf::from(&demo_dir).join("app/frontend/.env");
        let mut env_content = fs::read_to_string(&env_file).context("Failed to read .env file")?;
        env_content = env_content.replace("VITE_PROGRAM_PUBKEY=", &format!("VITE_PROGRAM_PUBKEY={}", program_pubkey));
        fs::write(&env_file, env_content).context("Failed to write to .env file")?;
    } else {
        // If program_pubkey is not empty, we need to find the corresponding key name
        graffiti_key_name = find_key_name_by_pubkey(&keys_file, &program_pubkey)?;
        println!("Using existing account with name: {}", graffiti_key_name);
    }

    // Get the program keypair from the keys.json file
    let program_keypair = get_keypair_from_name(&graffiti_key_name, &keys_file)?;
    let program_pubkey = Pubkey::from_slice(&program_keypair.public_key().serialize()[1..33]);

    // Deploy the program with the existing keypair
    deploy_program_from_path(
        &PathBuf::from(&demo_dir).join("app/program"),
        config,
        Some((program_keypair.clone(), program_pubkey)),
        get_rpc_url_with_fallback(args.rpc_url.clone(), config).unwrap(),
    ).await?;

    // Make the program executable
    let rpc_url = get_rpc_url_with_fallback(args.rpc_url.clone(), config).unwrap();
    make_program_executable(&program_keypair, &program_pubkey, rpc_url).await?;

    let graffiti_wall_state_exists = key_name_exists(&keys_file, "graffiti_wall_state")?;

    if graffiti_wall_state_exists {
        println!("  {} Using existing graffiti_wall_state account", "ℹ".bold().blue());
    } else {
        println!("  {} Creating new graffiti_wall_state account", "ℹ".bold().blue());
        create_account(&CreateAccountArgs {
            name: "graffiti_wall_state".to_string(),
            program_id: Some(hex::encode(program_pubkey.serialize())),
            rpc_url: Some(args.rpc_url.clone().unwrap_or_default()),
        }, config).await?;
    }

    // Get the public key of the graffiti_wall_state account
    let graffiti_wall_state_pubkey = get_pubkey_from_name("graffiti_wall_state", &keys_file)?;

    // Write the graffiti_wall_state public key into the app/frontend/.env file
    let env_file = PathBuf::from(&demo_dir).join("app/frontend/.env");
    let mut env_content = fs::read_to_string(&env_file).context("Failed to read .env file")?;
    env_content = env_content.replace("VITE_WALL_ACCOUNT_PUBKEY=", &format!("VITE_WALL_ACCOUNT_PUBKEY={}", graffiti_wall_state_pubkey));
    fs::write(&env_file, env_content).context("Failed to write to .env file")?;

    if !args.skip_cleanup {
        // Stop existing demo containers
        println!(
            "  {} Stopping any existing demo containers...",
            "→".bold().blue()
        );

        // Change to the demo directory
        std::env::set_current_dir(&demo_dir).context("Failed to change to demo directory")?;

        let stop_output = ShellCommand::new("docker-compose")
            .arg("-f")
            .arg("app/demo-docker-compose.yml")
            .arg("down")
            .output()
            .context("Failed to stop existing demo containers")?;

        if !stop_output.status.success() {
            println!(
                "  {} Warning: Failed to stop existing demo containers. Proceeding anyway.",
                "⚠".bold().yellow()
            );
        } else {
            println!(
                "  {} Existing demo containers stopped successfully",
                "✓".bold().green()
            );
        }

        // Remove the arch-network
        println!("  {} Removing arch-network...", "→".bold().blue());
        let remove_network_output = ShellCommand::new("docker")
            .args(&["network", "rm", "arch-network"])
            .output()
            .context("Failed to remove arch-network")?;

        if !remove_network_output.status.success() {
            let error_message = String::from_utf8_lossy(&remove_network_output.stderr);
            if !error_message.contains("not found") {
                println!("  {} Warning: Failed to remove arch-network: {}", "⚠".bold().yellow(), error_message);
            }
        }

        println!("  {} arch-network removed", "✓".bold().green());
    } else {
        println!(
            "  {} Skipping cleanup of existing containers and network",
            "ℹ".bold().blue()
        );
    }

    // Remove the arch-network
    println!("  {} Removing arch-network...", "→".bold().blue());
    let remove_network_output = ShellCommand::new("docker")
        .args(&["network", "rm", "arch-network"])
        .output()
        .context("Failed to remove arch-network")?;

    if !remove_network_output.status.success() {
        let error_message = String::from_utf8_lossy(&remove_network_output.stderr);
        if !error_message.contains("not found") {
            println!("  {} Warning: Failed to remove arch-network: {}", "⚠".bold().yellow(), error_message);
        }
    }

    println!("  {} arch-network removed", "✓".bold().green());

    // Create the arch-network if it doesn't exist
    println!("  {} Creating arch-network...", "→".bold().blue());
    let create_network_output = ShellCommand::new("docker")
        .args(&["network", "create", "arch-network"])
        .output()
        .context("Failed to create arch-network")?;

    if !create_network_output.status.success() {
        let error_message = String::from_utf8_lossy(&create_network_output.stderr);
        if !error_message.contains("already exists") {
            return Err(anyhow!("Failed to create arch-network: {}", error_message));
        }
    }

    println!("  {} arch-network created or already exists", "✓".bold().green());

    // Creating longer-lived values for the environment variables to avoid temporary value drop errors
    let program_pubkey_str = hex::encode(program_pubkey.serialize());
    let graffiti_wall_state_pubkey_str = graffiti_wall_state_pubkey.to_string();
    let demo_frontend_port_str = "5173".to_string();
    let indexer_port_str = "5175".to_string();

    let env_vars = vec![
        ("VITE_PROGRAM_PUBKEY", &program_pubkey_str),
        ("VITE_WALL_ACCOUNT_PUBKEY", &graffiti_wall_state_pubkey_str),
        ("DEMO_FRONTEND_PORT", &demo_frontend_port_str),
        ("INDEXER_PORT", &indexer_port_str),
    ];

    // Start the demo application
    println!("  {} Starting demo containers...", "→".bold().blue());

    // Create the docker-compose command with environment variables
    let mut command = ShellCommand::new("docker-compose");
    command
        .arg("-f")
        .arg("app/demo-docker-compose.yml")
        .arg("up")
        .arg("--build")
        .arg("-d");

    // Add environment variables to the command
    for (key, value) in env_vars {
        command.env(key, value);
    }

    let start_output = command
        .output()
        .context("Failed to start the demo application using Docker Compose")?;

    if !start_output.status.success() {
        return Err(anyhow!(
            "Failed to start the demo application: {}",
            String::from_utf8_lossy(&start_output.stderr)
        ));
    }

    println!(
        "{}",
        "Demo application started successfully!".bold().green()
    );

    // Open the browser with the demo application
    if let Err(e) = open_browser(webbrowser::Browser::Default, &format!("http://localhost:5173")) {
        return Err(anyhow!("Failed to open the browser: {}", e));
    }

    Ok(())
}

async fn start_gcp_demo(args: &DemoStartArgs, config: &Config) -> Result<()> {
    println!("Starting GCP deployment...");

    // Setup demo environment first
    let (demo_dir, _, _, rpc_url) = setup_demo_environment(args, config).await?;

    let project_id = args.gcp_project.clone()
        .ok_or_else(|| anyhow!("GCP project ID is required for GCP deployment"))?;

    // Build and deploy the demo container
    println!("Building and deploying demo container...");

    let demo_app_dir = demo_dir.join("app");

    println!("  {} Changing to demo app directory: {:?}", "→".bold().blue(), demo_app_dir);

    std::env::set_current_dir(&demo_app_dir)
        .context("Failed to change to demo app directory")?;

    // Build the container
    let image_name = format!("gcr.io/{}/arch-demo", project_id);
    let build_status = Command::new("docker")
        .args([
            "build",
            "-t", &image_name,
            "-f", "Dockerfile.cloudrun",
            "."
        ])
        .status()
        .context("Failed to build demo container")?;

    if !build_status.success() {
        return Err(anyhow!("Failed to build demo container"));
    }

    // Push to GCR
    let push_status = Command::new("docker")
        .args(["push", &image_name])
        .status()
        .context("Failed to push demo container")?;

    if !push_status.success() {
        return Err(anyhow!("Failed to push demo container"));
    }

    // Deploy to Cloud Run
    let deploy_status = Command::new("gcloud")
        .args([
            "run", "deploy", "arch-demo",
            "--image", &image_name,
            "--platform", "managed",
            "--region", "us-central1",
            "--port", "8080",
            "--allow-unauthenticated",
            "--project", &project_id,
            "--set-env-vars", &format!("ARCH_RPC_URL={}", rpc_url),
        ])
        .status()
        .context("Failed to deploy to Cloud Run")?;

    if !deploy_status.success() {
        return Err(anyhow!("Failed to deploy to Cloud Run"));
    }

    println!("✓ Demo application deployed successfully to Cloud Run");
    Ok(())
}

async fn deploy_to_cloud_run(project_id: &str, region: &str, demo_dir: &Path) -> Result<()> {
    // Build and push Docker image
    let image_name = format!("gcr.io/{}/arch-demo", project_id);

    println!("  {} Building Docker image...", "→".bold().blue());
    let build_output = ShellCommand::new("docker")
        .args(["build", "--platform", "linux/amd64", "-t", &image_name, "."])
        .current_dir(demo_dir.join("app/frontend"))
        .output()?;

    if !build_output.status.success() {
        return Err(anyhow!("Failed to build Docker image"));
    }

    println!("  {} Pushing image to Container Registry...", "→".bold().blue());
    let push_output = ShellCommand::new("docker")
        .args(["push", &image_name])
        .output()?;

    if !push_output.status.success() {
        return Err(anyhow!("Failed to push Docker image"));
    }

    println!("  {} Deploying to Cloud Run...", "→".bold().blue());
    let deploy_output = ShellCommand::new("gcloud")
        .args([
            "run", "deploy", "arch-demo",
            "--image", &image_name,
            "--platform", "managed",
            "--region", region,
            "--project", project_id,
            "--allow-unauthenticated"
        ])
        .output()?;

    if !deploy_output.status.success() {
        return Err(anyhow!("Failed to deploy to Cloud Run"));
    }

    Ok(())
}

fn find_key_name_by_pubkey(keys_file: &PathBuf, pubkey: &str) -> Result<String> {
    let keys = load_keys(keys_file)?;
    for (name, key_info) in keys.as_object().unwrap() {
        if key_info["public_key"].as_str().unwrap() == pubkey {
            return Ok(name.clone());
        }
    }
    Err(anyhow!("No key found with the given public key"))
}

fn get_pubkey_from_name(name: &str, keys_file: &Path) -> Result<String> {
    let keys = serde_json::from_str::<serde_json::Value>(&fs::read_to_string(keys_file)?)?;
    let pubkey = keys.get(name).context(format!("Key with name '{}' not found", name))?;
    let pubkey = pubkey.get("public_key").context(format!("Public key for '{}' not found", name))?;
    Ok(pubkey.as_str().context(format!("Public key for '{}' is not a string", name))?.to_string())
}

fn get_keypair_from_name(name: &str, keys_file: &PathBuf) -> Result<Keypair> {
    let keys = load_keys(keys_file)?;

    let key_info = keys.as_object()
        .and_then(|obj| obj.get(name))
        .ok_or_else(|| anyhow!("Key with name '{}' not found", name))?;

    let secret_key_hex = key_info["secret_key"]
        .as_str()
        .ok_or_else(|| anyhow!("Invalid secret key format for key '{}'", name))?;

    let secret_key = SecretKey::from_str(secret_key_hex)?;
    let secp = Secp256k1::new();
    let keypair = secp256k1::Keypair::from_secret_key(&secp, &secret_key);

    Ok(keypair)
}

pub async fn demo_stop(config: &Config) -> Result<()> {
    println!("{}", "Stopping the demo application...".bold().green());

    // Get the selected network from the config
    let selected_network = config.get_string("selected_network")
        .unwrap_or_else(|_| "development".to_string());

    // Set environment variables for the selected network
    set_env_vars(config, &selected_network)?;

    // Get the project directory from the config
    let project_dir = config
        .get_string("project.directory")
        .context("Failed to get project directory from config")?;

    // Change to the demo directory
    let demo_dir = PathBuf::from(project_dir).join("projects/demo");
    std::env::set_current_dir(&demo_dir).context("Failed to change to demo directory")?;

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
    println!("{}", "Current Configuration:".bold().green());
    println!();

    // Get config file path
    let config_path = get_config_path()?;
    let config_content = std::fs::read_to_string(&config_path)?;

    // Parse TOML content
    let parsed_config = toml_edit::Document::from_str(&config_content)?;

    // Helper function to print a section
    fn print_section(name: &str, table: &toml_edit::Table) {
        println!("{}", format!("[{}]", name).bold().blue());
        for (key, value) in table.iter() {
            match value {
                toml_edit::Item::Table(subtable) => {
                    println!();
                    print_section(&format!("{}.{}", name, key), subtable);
                }
                toml_edit::Item::Value(val) => {
                    let value_str = val.to_string();
                    if !value_str.is_empty() {
                        // Handle string values with proper quotes
                        let formatted_value = if value_str.starts_with('"') && !value_str.ends_with('"') {
                            format!("{}\"", value_str)
                        } else {
                            value_str
                        };
                        println!("  {} = {}",
                            key.to_string().yellow(),
                            formatted_value.bright_white()
                        );
                    }
                }
                _ => {}
            }
        }
        println!();
    }

    // Print each top-level section
    for (section_name, section) in parsed_config.as_table().iter() {
        if let toml_edit::Item::Table(table) = section {
            print_section(section_name, table);
        }
    }

    // Print config file location
    println!("{}", "Config file location:".bold().green());
    println!("  {}", config_path.display().to_string().bright_white());

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
        let confirm = Confirm::new()
            .with_prompt("Existing configuration found. Are you sure you want to reset it? This will remove all custom settings.")
            .default(false)
            .interact()?;

        if !confirm {
            println!("  {} Configuration reset cancelled", "ℹ".bold().blue());
            return Ok(());
        }

        // Backup the existing config file
        let backup_path = config_path.with_extension("toml.backup");
        if let Err(e) = fs::copy(&config_path, &backup_path) {
            println!(
                "  {} Warning: Failed to create backup: {}",
                "⚠".bold().yellow(),
                e
            );
        } else {
            println!(
                "  {} Created backup at {}",
                "✓".bold().green(),
                backup_path.display()
            );
        }
    }

    // Create the config directory if it doesn't exist
    fs::create_dir_all(config_dir).context("Failed to create config directory")?;

    // Get the default config content
    let default_config_content = include_str!("../templates/config.default.toml");

    // Write the default config
    fs::write(&config_path, default_config_content)
        .context("Failed to write default configuration")?;

    println!(
        "  {} Configuration reset to default at {}",
        "✓".bold().green(),
        config_path.display()
    );

    // Ensure all template files are reset as well
    println!("  {} Resetting template files...", "→".bold().blue());
    copy_template_files()?;

    println!("\n{}", "Configuration reset complete!".bold().green());
    println!(
        "  {} Use {} to view or {} to edit the new configuration",
        "ℹ".bold().blue(),
        "arch-cli config view".cyan(),
        "arch-cli config edit".cyan()
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

    let rpc_url = get_rpc_url_with_fallback(args.rpc_url.clone(), config).unwrap();
    println!("  {} RPC URL: {}", "ℹ".bold().blue(), rpc_url.yellow());

    // Get account address
    let account_address = generate_account_address(&rpc_url, caller_pubkey).await?;

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
        "5000".yellow()
    );
    println!("  {} Waiting for funds...", "⏳".bold().blue());

    create_arch_account(
        &caller_keypair,
        &caller_pubkey,
        &account_address,
        &wallet_manager,
        config,
        Some(args.rpc_url.clone().unwrap_or_default()),
    )
    .await?;

    // Only transfer ownership if program_id is provided
    if let Some(hex_program_id) = &args.program_id {
        if !hex_program_id.is_empty() {
            let program_id_bytes = hex::decode(hex_program_id)
                .context("Failed to decode program ID from hex")?;
            let program_id = Pubkey::from_slice(&program_id_bytes);
            
            let rpc_url = get_rpc_url_with_fallback(args.rpc_url.clone(), config).unwrap();
            
            // Transfer ownership to the program
            transfer_account_ownership(
                &caller_keypair,
                &caller_pubkey,
                &program_id,
                rpc_url,
            ).await?;
            
            println!(
                "{}",
                "Account created and ownership transferred successfully!"
                    .bold()
                    .green()
            );
        } else {
            println!(
                "{}",
                "Account created successfully!"
                    .bold()
                    .green()
            );
        }
    } else {
        println!(
            "{}",
            "Account created successfully!"
                .bold()
                .green()
        );
    }

    // Save the account information to keys.json
    save_keypair_to_json(&keys_file, &caller_keypair, &caller_pubkey, &args.name)?;

    // Output the private key to the user
    let private_key_hex = hex::encode(secret_key.secret_bytes());
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
        hex::encode(caller_pubkey.serialize()).bright_green()
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
        println!("  {} Account: {}", "→".bold().blue(), name.yellow());
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
    let keys_dir = get_config_dir()?;  // Changed from ensure_keys_dir()
    let keys_file = keys_dir.join("keys.json");

    if !keys_file.exists() {
        println!("  {} No accounts found", "ℹ".bold().blue());
        return Ok(());
    }

    let file = OpenOptions::new().read(true).open(&keys_file)?;
    let reader = BufReader::new(file);
    let mut accounts: Value = serde_json::from_reader(reader)?;

    let accounts_obj = accounts.as_object_mut().unwrap();
    let mut account_to_remove = None;
    let mut account_name = String::new();

    // First try to find by public key (account_id)
    if accounts_obj.contains_key(&args.identifier) {
        account_to_remove = Some(args.identifier.clone());
        account_name = accounts_obj[&args.identifier]["name"]
            .as_str()
            .unwrap_or(&args.identifier)
            .to_string();
    } else {
        // If not found, try to find by name
        for (account_id, account_info) in accounts_obj.iter() {
            if let Some(name) = account_info["name"].as_str() {
                if name == args.identifier {
                    account_to_remove = Some(account_id.clone());
                    account_name = name.to_string();
                    break;
                }
            }
        }
    }

    if let Some(account_id) = account_to_remove {
        // Use dialoguer for better user interaction
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to delete account '{}' (public key: {})?",
                account_name, account_id
            ))
            .default(false)
            .interact()?;

        if confirm {
            accounts_obj.remove(&account_id);
            let file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&keys_file)?;
            serde_json::to_writer_pretty(file, &accounts)?;
            println!(
                "  {} Account '{}' deleted successfully",
                "✓".bold().green(),
                account_name
            );
        } else {
            println!(
                "  {} Deletion of account '{}' cancelled",
                "✗".bold().red(),
                account_name
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

async fn generate_account_address(rpc_url: &str, caller_pubkey: Pubkey) -> Result<String> {
    let rpc_url_clone = rpc_url.to_string();

    // Get program account address from network
    let account_address = tokio::task::spawn_blocking(move || {
        get_account_address(&rpc_url_clone, caller_pubkey)
    }).await.unwrap();

    println!("  {} Account address: {}", "ℹ".bold().blue(), account_address.yellow());

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
    rpc_url: Option<String>,
) -> Result<()> {
    let tx_info = fund_address(&wallet_manager.client, account_address, config).await?;

    if let Some(info) = tx_info {
        let caller_keypair = caller_keypair.clone();
        let caller_pubkey = *caller_pubkey;
        let rpc_url = get_rpc_url_with_fallback(rpc_url, config).unwrap();

        let (txid, _) = tokio::task::spawn_blocking(move || {
            sign_and_send_instruction(
                SystemInstruction::new_create_account_instruction(
                    hex::decode(&info.info.txid.to_string())
                        .unwrap()
                        .try_into()
                        .unwrap(),
                    0,
                    caller_pubkey,
                ),
                vec![caller_keypair],
                rpc_url,
            )
            .expect("signing and sending a transaction should not fail")
        })
        .await
        .unwrap();

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

        Ok(())
    }
}

async fn transfer_account_ownership(
    caller_keypair: &Keypair,
    account_pubkey: &Pubkey,
    program_pubkey: &Pubkey,
    rpc_url: String,
) -> Result<()> {
    let mut instruction_data = vec![3]; // Transfer instruction
    instruction_data.extend(program_pubkey.serialize());

    println!(
        "  {} Account public key: {:?}",
        "ℹ".bold().blue(),
        hex::encode(account_pubkey.serialize())
    );

    let instruction_data_clone = instruction_data.clone();
    let account_pubkey_clone = *account_pubkey;
    let caller_keypair_clone = caller_keypair.clone();

    let (_txid, _) = tokio::task::spawn_blocking(move || {
        sign_and_send_instruction(
            Instruction {
                program_id: Pubkey::system_program(),
                accounts: vec![AccountMeta {
                    pubkey: account_pubkey_clone,
                    is_signer: true,
                    is_writable: true,
                }],
                data: instruction_data_clone,
            },
            vec![caller_keypair_clone],
            rpc_url,
        )
        .expect("signing and sending a transaction should not fail")
    })
    .await
    .unwrap();

    Ok(())
}

pub async fn indexer_start(args: &IndexerStartArgs, config: &Config) -> Result<()> {
    match args.target.as_str() {
        "local" => start_local_indexer(config).await,
        "gcp" => start_gcp_indexer(args, config).await,
        _ => Err(anyhow!("Invalid deployment target. Use 'local' or 'gcp'"))
    }
}

pub async fn start_local_indexer(config: &Config) -> Result<()> {
    println!("{}", "Starting the arch-indexer...".bold().green());

    let arch_node_url = "http://host.docker.internal:9002";

    // Get the selected network from the config
    let selected_network = config.get_string("selected_network")
        .unwrap_or_else(|_| "development".to_string());

    // Set environment variables for the selected network
    set_env_vars(config, &selected_network)?;

    // Get or create the directory for the indexer
    let indexer_dir = get_indexer_dir()?;

    // Clone or update the arch-indexer repository
    clone_or_update_repo(&indexer_dir)?;

    // Remember the current directory
    let original_dir = env::current_dir()?;

    // Change to the indexer directory
    env::set_current_dir(&indexer_dir)
        .context("Failed to change to indexer directory")?;

    // Start the indexer using docker-compose
    let output = ShellCommand::new("docker-compose")
        .arg("-f")
        .arg("docker-compose.yml")
        .arg("up")
        .arg("--build")
        .arg("-d")
        .env("ARCH_NODE_URL", arch_node_url)
        .output()
        .context("Failed to start the arch-indexer using Docker Compose")?;

    // Change back to the original directory
    env::set_current_dir(original_dir)
        .context("Failed to change back to original directory")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to start the arch-indexer: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("{}", "arch-indexer started successfully!".bold().green());
    Ok(())
}

async fn prepare_indexer_files(temp_dir: &Path) -> Result<()> {
    println!("  {} Preparing indexer files...", "→".bold().blue());

    // Clone the repository
    let clone_status = Command::new("git")
        .args([
            "clone",
            "https://github.com/arch-network/arch-indexer.git",
            temp_dir.to_str().unwrap()
        ])
        .status()
        .context("Failed to clone indexer repository")?;

    if !clone_status.success() {
        return Err(anyhow!("Failed to clone indexer repository"));
    }

    // Create docker-compose.yml for GCP
    let docker_compose = r#"version: '3'
services:
  indexer:
    build: .
    environment:
      - DB_USER=postgres
      - DB_HOST=127.0.0.1
      - DB_NAME=archindexer
      - DB_PASSWORD=${DB_PASSWORD}
      - DB_PORT=5432
      - ARCH_NODE_URL=${ARCH_NODE_URL}
    ports:
      - "5175:5175"
    restart: always

  db:
    image: postgres:13
    environment:
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=archindexer
    volumes:
      - pgdata:/var/lib/postgresql/data
    restart: always

volumes:
  pgdata:"#;

    fs::write(temp_dir.join("docker-compose.yml"), docker_compose)?;

    // Create Dockerfile
    let dockerfile = r#"FROM node:18-slim
WORKDIR /usr/src/app
COPY package*.json ./
RUN npm install
COPY . .
EXPOSE 5175
CMD ["node", "src/index.js"]"#;

    fs::write(temp_dir.join("Dockerfile"), dockerfile)?;

    // Create init.sql
    let init_sql = r#"CREATE TABLE IF NOT EXISTS blocks (
    height BIGINT PRIMARY KEY,
    hash TEXT NOT NULL,
    timestamp BIGINT,
    bitcoin_block_height BIGINT
);

CREATE TABLE IF NOT EXISTS transactions (
    txid TEXT PRIMARY KEY,
    block_height BIGINT REFERENCES blocks(height),
    data JSONB,
    status INTEGER,
    bitcoin_txids TEXT[]
);"#;

    fs::write(temp_dir.join("init.sql"), init_sql)?;

    Ok(())
}

async fn stop_gcp_indexer(args: &IndexerStartArgs) -> Result<()> {
    let project_id = args.gcp_project.as_ref()
        .ok_or_else(|| anyhow!("GCP project ID is required"))?;
    let zone = &"us-central1".to_string();
    let region = args.gcp_region.as_ref().unwrap_or(zone);

    println!("{}", "Stopping GCP indexer...".bold().green());

    // Stop and delete the indexer instance
    let _ = ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "delete", "arch-indexer",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--quiet"
        ])
        .output()?;

    // Stop and delete the proxy instance
    let _ = ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "delete", "arch-indexer-proxy",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--quiet"
        ])
        .output()?;

    // Delete Cloud SQL instance
    let _ = ShellCommand::new("gcloud")
        .args([
            "sql", "instances", "delete", "arch-indexer-db",
            "--project", project_id,
            "--quiet"
        ])
        .output()?;

    println!("{}", "GCP indexer stopped successfully!".bold().green());
    Ok(())
}

pub async fn start_gcp_indexer(args: &IndexerStartArgs, config: &Config) -> Result<()> {
    let project_id = args.gcp_project.as_ref()
        .ok_or_else(|| anyhow!("GCP project ID is required for GCP deployment"))?;
    let zone = &"us-central1".to_string();
    let machine = &"e2-medium".to_string();
    let region = args.gcp_region.as_ref().unwrap_or(zone);
    let machine_type = args.gcp_machine_type.as_ref().unwrap_or(machine);

    println!("Starting indexer deployment to GCP...");

    // Setup Cloud SQL
    let (sql_connection_name, db_password) = setup_cloud_sql(project_id, region).await?;

    // Initialize schema
    initialize_cloud_sql_schema(project_id, "arch-indexer-db").await?;

    let temp_dir = tempfile::tempdir()?;
    prepare_indexer_files(temp_dir.path()).await?;

    // Build and push using Cloud Build
    let cloudbuild_content = format!(r#"steps:
- name: 'gcr.io/cloud-builders/docker'
  args: ['build', '-t', 'gcr.io/{}/arch-indexer:latest', '.']
images: ['gcr.io/{}/arch-indexer:latest']
"#, project_id, project_id);

    fs::write(temp_dir.path().join("cloudbuild.yaml"), cloudbuild_content)?;

    println!("  {} Building and pushing indexer image...", "→".bold().blue());
    let build_output = ShellCommand::new("gcloud")
        .args([
            "builds", "submit",
            "--config", temp_dir.path().join("cloudbuild.yaml").to_str().unwrap(),
            "--project", project_id,
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .context("Failed to build and push image")?;

    if !build_output.status.success() {
        return Err(anyhow!(
            "Failed to build indexer image: {}",
            String::from_utf8_lossy(&build_output.stderr)
        ));
    }

    // Deploy the indexer container
    println!("  {} Deploying indexer to GCP...", "→".bold().blue());
    let rpc_url = args.rpc_url.as_deref().unwrap_or("http://localhost:9001");
    let create_instance_output = ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "create-with-container", "arch-indexer",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--machine-type", machine_type,
            "--container-image", &format!("gcr.io/{}/arch-indexer:latest", project_id),
            "--tags", "indexer",
            "--container-env", &format!("ARCH_NODE_URL={}", rpc_url),
            "--container-env", &format!("DB_HOST=/cloudsql/{}", sql_connection_name),
            "--container-env", "DB_USER=postgres",
            "--container-env", "DB_NAME=archindexer",
            "--container-env", &format!("DB_PASSWORD={}", db_password),
            "--container-env", "DB_PORT=5432",
            "--container-mount-host-path=mount-path=/cloudsql,host-path=/cloudsql,mode=rw",
        ])
        .output()
        .context("Failed to create indexer instance")?;

    // Rest of the function (SSL proxy setup) remains the same
    let indexer_ip = String::from_utf8_lossy(&ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "describe", "arch-indexer",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--format", "get(networkInterfaces[0].networkIP)"
        ])
        .output()?
        .stdout).trim().to_string();

    setup_indexer_ssl_proxy(project_id, region, &indexer_ip).await?;

    Ok(())
}

async fn setup_cloud_sql(project_id: &str, region: &str) -> Result<(String, String)> {
    println!("  {} Setting up Cloud SQL instance...", "→".bold().blue());

    let instance_name = "arch-indexer-db";
    let db_password = generate_random_password();

    // Check if instance exists
    println!("  {} Checking if Cloud SQL instance exists...", "→".bold().blue());
    let instance_exists = ShellCommand::new("gcloud")
        .args([
            "sql", "instances", "describe", instance_name,
            "--project", project_id,
        ])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !instance_exists {
        println!("  {} Creating new Cloud SQL instance...", "→".bold().blue());
        let create_output = ShellCommand::new("gcloud")
            .args([
                "sql", "instances", "create", instance_name,
                "--project", project_id,
                "--database-version", "POSTGRES_13",
                "--tier", "db-f1-micro",
                "--region", region,
                "--root-password", &db_password,
                "--availability-type", "ZONAL",
            ])
            .output()
            .context("Failed to execute gcloud sql instances create command")?;

        if !create_output.status.success() {
            return Err(anyhow!(
                "Failed to create Cloud SQL instance: {}",
                String::from_utf8_lossy(&create_output.stderr)
            ));
        }

        // Create database
        println!("  {} Creating database...", "→".bold().blue());
        let db_output = ShellCommand::new("gcloud")
            .args([
                "sql", "databases", "create", "archindexer",
                "--instance", instance_name,
                "--project", project_id,
            ])
            .output()
            .context("Failed to execute gcloud sql databases create command")?;

        if !db_output.status.success() {
            return Err(anyhow!(
                "Failed to create database: {}",
                String::from_utf8_lossy(&db_output.stderr)
            ));
        }

        // Create user
        println!("  {} Creating database user...", "→".bold().blue());
        let user_output = ShellCommand::new("gcloud")
            .args([
                "sql", "users", "create", "postgres",
                "--instance", instance_name,
                "--project", project_id,
                "--password", &db_password,
            ])
            .output()
            .context("Failed to execute gcloud sql users create command")?;

        if !user_output.status.success() {
            return Err(anyhow!(
                "Failed to create database user: {}",
                String::from_utf8_lossy(&user_output.stderr)
            ));
        }
    } else {
        println!("  {} Using existing Cloud SQL instance", "✓".bold().green());
    }

    // Get connection name
    println!("  {} Getting connection details...", "→".bold().blue());
    let conn_output = ShellCommand::new("gcloud")
        .args([
            "sql", "instances", "describe", instance_name,
            "--project", project_id,
            "--format", "get(connectionName)",
        ])
        .output()
        .context("Failed to get Cloud SQL connection name")?;

    if !conn_output.status.success() {
        return Err(anyhow!(
            "Failed to get Cloud SQL connection name: {}",
            String::from_utf8_lossy(&conn_output.stderr)
        ));
    }

    let connection_name = String::from_utf8_lossy(&conn_output.stdout).trim().to_string();

    println!("  {} Cloud SQL setup complete", "✓".bold().green());
    Ok((connection_name, db_password))
}

async fn initialize_cloud_sql_schema(projectid: &str, instance_name: &str) -> Result<()> {
    println!("  {} Initializing database schema...", "→".bold().blue());

    let temp_file = tempfile::NamedTempFile::new()?;

    // Use the same schema as in prepare_indexer_files
    let init_sql = r#"CREATE TABLE IF NOT EXISTS blocks (
    height INTEGER PRIMARY KEY,
    hash TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    bitcoin_block_height INTEGER
);

CREATE TABLE IF NOT EXISTS transactions (
    txid TEXT PRIMARY KEY,
    block_height INTEGER NOT NULL,
    data JSONB NOT NULL,
    status INTEGER NOT NULL DEFAULT 0,
    bitcoin_txids TEXT[] DEFAULT '{}',
    FOREIGN KEY (block_height) REFERENCES blocks(height)
);

CREATE INDEX IF NOT EXISTS idx_transactions_block_height ON transactions(block_height);
CREATE INDEX IF NOT EXISTS idx_blocks_bitcoin_block_height ON blocks(bitcoin_block_height);"#;

    fs::write(&temp_file, init_sql)?;

    let import_output = ShellCommand::new("gcloud")
        .args([
            "sql", "import", "sql",
            instance_name,
            temp_file.path().to_str().unwrap(),
            "--project", projectid,
            "--database", "archindexer",
        ])
        .output()
        .context("Failed to import SQL schema")?;

    if !import_output.status.success() {
        return Err(anyhow!("Failed to initialize database schema"));
    }

    Ok(())
}

async fn setup_indexer_ssl_proxy(project_id: &str, region: &str, indexer_ip: &str) -> Result<()> {
    println!("  {} Setting up HTTPS proxy for indexer...", "→".bold().blue());

    let temp_dir = tempfile::tempdir()?;

    // Create nginx.conf for indexer
    let nginx_conf = format!(r#"
events {{
    worker_connections 1024;
}}
http {{
    error_log /dev/stderr debug;
    access_log /dev/stdout combined;

    proxy_connect_timeout 60;
    proxy_send_timeout 60;
    proxy_read_timeout 60;

    # SSL settings
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers off;

    # CORS settings
    map $request_method $cors_method {{
        OPTIONS 'true';
        default 'false';
    }}

    server {{
        listen 443 ssl;
        server_name _;

        ssl_certificate /etc/nginx/ssl/nginx.crt;
        ssl_certificate_key /etc/nginx/ssl/nginx.key;

        # Global CORS headers
        add_header 'Access-Control-Allow-Origin' '*' always;
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS' always;
        add_header 'Access-Control-Allow-Headers' 'DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,Authorization' always;
        add_header 'Access-Control-Expose-Headers' 'Content-Length,Content-Range' always;

        location / {{
            if ($cors_method = 'true') {{
                add_header 'Access-Control-Max-Age' 1728000;
                add_header 'Content-Type' 'text/plain charset=UTF-8';
                add_header 'Content-Length' 0;
                return 204;
            }}

            proxy_pass http://{}:5175;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;

            # Handle SSL termination properly
            proxy_ssl_server_name on;
            proxy_redirect off;

            # Additional security headers
            add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
            add_header X-Content-Type-Options "nosniff" always;
            add_header X-Frame-Options "SAMEORIGIN" always;
            add_header X-XSS-Protection "1; mode=block" always;
        }}
    }}
}}
"#, indexer_ip);

    // Write nginx.conf (your existing config is good)
    fs::write(temp_dir.path().join("nginx.conf"), &nginx_conf)?;

    // Create Dockerfile for SSL proxy
    let dockerfile_content = r#"FROM --platform=linux/amd64 nginx:alpine
COPY nginx.conf /etc/nginx/nginx.conf
RUN mkdir -p /etc/nginx/ssl
RUN apk add --no-cache openssl
RUN openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout /etc/nginx/ssl/nginx.key \
    -out /etc/nginx/ssl/nginx.crt \
    -subj "/CN=arch-indexer/O=Arch Network/C=US"
EXPOSE 443
"#;

    fs::write(temp_dir.path().join("Dockerfile"), dockerfile_content)?;

    // Build and push the proxy image
    let proxy_image = format!("gcr.io/{}/arch-indexer-proxy:latest", project_id);

    println!("  {} Building and pushing proxy image...", "→".bold().blue());
    let build_status = Command::new("docker")
        .args([
            "build",
            "-t", &proxy_image,
            temp_dir.path().to_str().unwrap(),
        ])
        .status()
        .context("Failed to build proxy image")?;

    if !build_status.success() {
        return Err(anyhow!("Failed to build proxy image"));
    }

    let push_status = Command::new("docker")
        .args(["push", &proxy_image])
        .status()
        .context("Failed to push proxy image")?;

    if !push_status.success() {
        return Err(anyhow!("Failed to push proxy image"));
    }

    // Create firewall rules
    println!("  {} Creating firewall rules...", "→".bold().blue());
    let _ = ShellCommand::new("gcloud")
        .args([
            "compute", "firewall-rules", "create", "allow-indexer-internal",
            "--project", project_id,
            "--allow", "tcp:5175",
            "--source-tags", "indexer-proxy",
            "--target-tags", "indexer",
            "--description", "Allow proxy to indexer communication",
        ])
        .output();

    let _ = ShellCommand::new("gcloud")
        .args([
            "compute", "firewall-rules", "create", "allow-indexer-https",
            "--project", project_id,
            "--allow", "tcp:443",
            "--target-tags", "indexer-proxy",
            "--description", "Allow incoming HTTPS traffic for indexer proxy",
        ])
        .output();

    // Deploy the proxy container
    println!("  {} Deploying HTTPS proxy...", "→".bold().blue());
    let create_proxy_output = ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "create-with-container", "arch-indexer-proxy",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--machine-type", "e2-micro",
            "--container-image", &proxy_image,
            "--tags", "indexer-proxy",
        ])
        .output()
        .context("Failed to create proxy instance")?;

    if !create_proxy_output.status.success() {
        return Err(anyhow!(
            "Failed to create proxy instance: {}",
            String::from_utf8_lossy(&create_proxy_output.stderr)
        ));
    }

    // Get the proxy's external IP
    let proxy_ip = String::from_utf8_lossy(&ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "describe", "arch-indexer-proxy",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--format", "get(networkInterfaces[0].accessConfigs[0].natIP)"
        ])
        .output()?
        .stdout).trim().to_string();

    println!("\n{}", "HTTPS proxy setup complete!".bold().green());
    println!("Proxy IP: {}", proxy_ip);
    println!("HTTPS endpoint: {}", format!("https://{}", proxy_ip).yellow());

    Ok(())
}

fn generate_random_password() -> String {
    // Generate a random password with:
    // - Length of 16 characters
    // - Mix of uppercase, lowercase, numbers
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

fn get_indexer_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    let indexer_dir = config_dir.join("arch-indexer");
    fs::create_dir_all(&indexer_dir)?;
    Ok(indexer_dir)
}

fn clone_or_update_repo(indexer_dir: &Path) -> Result<()> {
    if indexer_dir.join(".git").exists() {
        // Repository already exists, update it
        println!("  {} Updating arch-indexer repository...", "→".bold().blue());
        let status = ShellCommand::new("git")
            .current_dir(indexer_dir)
            .args(&["pull", "origin", "main"])
            .status()
            .context("Failed to update arch-indexer repository")?;

        if !status.success() {
            return Err(anyhow!("Failed to update arch-indexer repository"));
        }
    } else {
        // Clone the repository
        println!("  {} Cloning arch-indexer repository...", "→".bold().blue());
        let status = ShellCommand::new("git")
            .args(&[
                "clone",
                "https://github.com/Arch-Network/arch-indexer.git",
                indexer_dir.to_str().unwrap(),
            ])
            .status()
            .context("Failed to clone arch-indexer repository")?;

        if !status.success() {
            return Err(anyhow!("Failed to clone arch-indexer repository"));
        }
    }

    println!("  {} arch-indexer repository ready", "✓".bold().green());
    Ok(())
}

pub async fn indexer_stop(args: &IndexerStartArgs, config: &Config) -> Result<()> {
    match args.target.as_str() {
        "local" => stop_local_indexer(config).await,
        "gcp" => stop_gcp_indexer(args).await,
        _ => Err(anyhow!("Invalid deployment target. Use 'local' or 'gcp'"))
    }
}

pub async fn stop_local_indexer(config: &Config) -> Result<()> {
    println!("{}", "Stopping the arch-indexer...".bold().green());

    // Get the selected network from the config
    let selected_network = config.get_string("selected_network")
        .unwrap_or_else(|_| "development".to_string());

    // Set environment variables for the selected network
    set_env_vars(config, &selected_network)?;

    let indexer_dir = get_indexer_dir()?;

    // Remember the current directory
    let original_dir = env::current_dir()?;

    // Change to the indexer directory
    env::set_current_dir(&indexer_dir)
        .context("Failed to change to indexer directory")?;

    let output = ShellCommand::new("docker-compose")
        .arg("-f")
        .arg("docker-compose.yml")
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

    // Change back to the original directory
    env::set_current_dir(original_dir)
        .context("Failed to change back to original directory")?;

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

    // Get the selected network from the config
    let selected_network = config.get_string("selected_network")
        .unwrap_or_else(|_| "development".to_string());

    // Set environment variables for the selected network
    set_env_vars(config, &selected_network)?;

    // Stop and remove containers
    let output = Command::new("docker-compose")
        .arg("-f")
        .arg("arch-indexer/docker-compose.yml")
        .arg("down")
        .arg("-v") // This will also remove named volumes declared in the "volumes" section
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

    println!("{}", "Arch-indexer cleaned up successfully!".bold().green());
    Ok(())
}

pub async fn validator_start(args: &ValidatorStartArgs, config: &Config) -> Result<()> {
    match args.target.as_str() {
        "local" => start_local_validator(&args, config).await,
        "gcp" => start_gcp_validator(&args, config).await,
        _ => Err(anyhow!("Invalid deployment target. Use 'local' or 'gcp'"))
    }
}

async fn start_local_validator(args: &ValidatorStartArgs, config: &Config) -> Result<()> {
    println!("{}", "Starting the local validator...".bold().green());

    let _network = &args.network;
    let rust_log = config.get_string("arch.rust_log")?;
    let rpc_bind_ip = "0.0.0.0";
    let rpc_bind_port = config.get_string("arch.leader_rpc_port")?;
    let bitcoin_rpc_password = config.get_string("bitcoin_rpc_password")?;

    // Validate Bitcoin RPC endpoint format
    let bitcoin_rpc_endpoint = {
        let endpoint = config.get_string("bitcoin_rpc_endpoint")?;
        if endpoint.contains("://") || endpoint.contains("/") {
            return Err(anyhow!("Bitcoin RPC endpoint should not contain protocol (http://) or path. Expected format: domain"));
        }
        let endpoint_regex = regex::Regex::new(r"^[a-zA-Z0-9.-]+$")?;
        if !endpoint_regex.is_match(&endpoint) {
            return Err(anyhow!("Invalid Bitcoin RPC endpoint format. Expected format: domain (e.g., localhost)"));
        }
        endpoint
    };

    // Validate port number
    let bitcoin_rpc_port = {
        let port = config.get_string("bitcoin_rpc_port")?;
        port.parse::<u16>().map_err(|_| anyhow!("Invalid Bitcoin RPC port number"))?;
        port
    };

    // Validate credentials are not empty
    let bitcoin_rpc_username = {
        let username = config.get_string("bitcoin_rpc_user")?;
        if username.trim().is_empty() {
            return Err(anyhow!("Bitcoin RPC username cannot be empty"));
        }
        username
    };

    let container_name = "local_validator";
    let container_exists = String::from_utf8(
        ShellCommand::new("docker")
            .arg("ps")
            .arg("-a")
            .arg("--format")
            .arg("{{.Names}}")
            .output()
            .context("Failed to check existing containers")?
            .stdout,
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

async fn start_gcp_validator(args: &ValidatorStartArgs, config: &Config) -> Result<()> {
    let project_id = args.gcp_project.as_ref()
        .ok_or_else(|| anyhow!("GCP project ID is required for GCP deployment"))?;
    let region = args.gcp_region.as_ref()
        .map_or("us-central1".to_string(), |r| r.to_string());
    let machine_type = args.gcp_machine_type.as_ref()
        .map_or("e2-medium".to_string(), |m| m.to_string());
    let instance_name = "arch-validator";

    // Get network from ValidatorStartArgs, but if development then network is "devnet", if testnet then network is "testnet", if mainnet then network is "mainnet"
    let network = match args.network.as_str() {
        "development" => "devnet",
        "testnet" => "testnet",
        "mainnet" => "mainnet",
        _ => "devnet",
    }.to_string();
    println!("Network: {}", network.bold().green());

    println!("{}", "Starting validator deployment to GCP...".bold().green());

    // Check if instance already exists
    let instance_exists = ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "describe", instance_name,
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--format", "get(name)"
        ])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if instance_exists {
        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("A validator instance already exists. Would you like to recreate it?")
            .default(false)
            .interact()?;

        if !proceed {
            // Get the instance's external IP and display current status
            let describe_output = ShellCommand::new("gcloud")
                .args([
                    "compute", "instances", "describe", instance_name,
                    "--project", project_id,
                    "--zone", &format!("{}-a", region),
                    "--format", "get(networkInterfaces[0].accessConfigs[0].natIP,status)"
                ])
                .output()?;

            let info = String::from_utf8_lossy(&describe_output.stdout);
            let mut lines = info.lines();
            let ip = lines.next().unwrap_or("unknown");
            let status = lines.next().unwrap_or("unknown");

            println!("\n{}", "Current validator instance:".bold().blue());
            println!("Status: {}", status);
            println!("External IP: {}", ip);
            println!("RPC endpoint: {}", format!("http://{}:9001", ip).yellow());
            
            println!("\nTo view logs, run:");
            println!("  {}", format!("gcloud compute instances get-serial-port-output {} --zone {} --project {}", 
                instance_name, 
                &format!("{}-a", region),
                project_id
            ).cyan());
            
            return Ok(());
        }

        // Delete the existing instance
        println!("  {} Removing existing validator instance...", "→".bold().blue());
        let delete_output = ShellCommand::new("gcloud")
            .args([
                "compute", "instances", "delete", instance_name,
                "--project", project_id,
                "--zone", &format!("{}-a", region),
                "--quiet"  // Skip confirmation
            ])
            .output()
            .context("Failed to delete existing instance")?;

        if !delete_output.status.success() {
            return Err(anyhow!(
                "Failed to delete existing instance: {}",
                String::from_utf8_lossy(&delete_output.stderr)
            ));
        }
        println!("  {} Existing instance removed", "✓".bold().green());
    }

    // Create a temporary directory for the build
    let temp_dir = tempfile::tempdir()?;
    println!("  {} Creating build directory", "→".bold().blue());

    // Create Dockerfile
    let dockerfile_content = r#"FROM ghcr.io/arch-network/local_validator:latest

EXPOSE 9001

ENV RUST_LOG=info
ENV NETWORK_MODE=$network

ENTRYPOINT ["/usr/bin/local_validator"]
"#;

    let dockerfile_path = temp_dir.path().join("Dockerfile");
    fs::write(&dockerfile_path, dockerfile_content)?;
    println!("  {} Created Dockerfile", "✓".bold().green());

    // Create cloudbuild.yaml
    let cloudbuild_content = format!(r#"steps:
- name: 'gcr.io/cloud-builders/docker'
  args: ['build', '-t', 'gcr.io/{}/arch-validator:latest', '.']
images: ['gcr.io/{}/arch-validator:latest']
"#, project_id, project_id);

    let cloudbuild_path = temp_dir.path().join("cloudbuild.yaml");
    fs::write(&cloudbuild_path, cloudbuild_content)?;
    println!("  {} Created Cloud Build configuration", "✓".bold().green());

    // Build and push the validator image to Google Container Registry
    println!("Building and pushing validator image to GCR...");
    let build_push_output = ShellCommand::new("gcloud")
        .args([
            "builds", "submit",
            "--config", cloudbuild_path.to_str().unwrap(),
            "--project", project_id,
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .context("Failed to build and push image to GCR")?;

    let image_name = format!("gcr.io/{}/arch-validator:latest", project_id);

    println!("  {} Image built and pushed successfully", "✓".bold().green());

    // Create firewall rule if it doesn't exist
    println!("Ensuring firewall rule exists for validator...");
    let firewall_rule_name = "allow-validator";
    let create_firewall_output = ShellCommand::new("gcloud")
        .args([
            "compute", "firewall-rules", "create", firewall_rule_name,
            "--project", project_id,
            "--allow", "tcp:9001",
            "--target-tags", "validator",
            "--description", "Allow incoming traffic on port 9001 for validator",
        ])
        .output();

    // Ignore if firewall rule already exists
    if let Err(e) = create_firewall_output {
        println!("  {} Firewall rule may already exist: {}", "ℹ".bold().blue(), e);
    }

    // Create and start the GCE instance
    println!("Creating GCE instance for validator...");
    let instance_name = "arch-validator";
    let create_instance_output = ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "create-with-container", instance_name,
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--machine-type", &machine_type,
            "--container-image", &image_name,
            "--container-env",
            &format!("RUST_LOG=info,NETWORK_MODE={}", network),
            "--container-command=/usr/bin/local_validator",
            "--container-arg=--rpc-bind-ip=0.0.0.0",
            "--container-arg=--rpc-bind-port=9001",
            "--tags", "validator",
            &format!("--container-arg=--bitcoin-rpc-endpoint={}", 
                config.get_string("networks.development.bitcoin_rpc_endpoint")?),
            &format!("--container-arg=--bitcoin-rpc-port={}", 
                config.get_string("networks.development.bitcoin_rpc_port")?),
            &format!("--container-arg=--bitcoin-rpc-username={}", 
                config.get_string("networks.development.bitcoin_rpc_user")?),
            &format!("--container-arg=--bitcoin-rpc-password={}", 
                config.get_string("networks.development.bitcoin_rpc_password")?),
        ])
        .output()
        .context("Failed to create GCE instance")?;

    if !create_instance_output.status.success() {
        return Err(anyhow!(
            "Failed to create GCE instance: {}",
            String::from_utf8_lossy(&create_instance_output.stderr)
        ));
    }

    // Get the instance's external IP
    let describe_output = ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "describe", instance_name,
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--format", "get(networkInterfaces[0].accessConfigs[0].natIP)"
        ])
        .output()
        .context("Failed to get instance IP")?;

    let instance_ip = String::from_utf8_lossy(&describe_output.stdout).trim().to_string();

    println!("{}", "Validator deployed successfully to GCP!".bold().green());
    println!("Instance name: {}", instance_name);
    println!("Instance zone: {}", &format!("{}-a", region));
    println!("External IP: {}", instance_ip);
    println!("Validator RPC endpoint: {}", format!("http://{}:9001", instance_ip).yellow());

    println!("\n{}", "Setting up HTTPS access...".bold().blue());
    setup_ssl_proxy(project_id, &region, &instance_ip).await?;
    
    println!("\nTo view logs, run:");
    println!("  {}", format!("gcloud compute instances get-serial-port-output {} --zone {} --project {}", 
        instance_name, 
        &format!("{}-a", region),
        project_id
    ).cyan());
    
    println!("\nTo SSH into the instance, run:");
    println!("  {}", format!("gcloud compute ssh {} --zone {} --project {}", 
        instance_name, 
        &format!("{}-a", region),
        project_id
    ).cyan());

    Ok(())
}
// Update the validator_stop function signature and implementation
pub async fn validator_stop(args: &ValidatorStartArgs) -> Result<()> {
    println!("{}", "Stopping the validator...".bold().green());

    match args.target.as_str() {
        "local" => stop_local_validator(),
        "gcp" => {
            let project_id = args.gcp_project.as_ref()
                .ok_or_else(|| anyhow!("GCP project ID is required for GCP deployment"))?;
            let region = args.gcp_region.as_ref()
                .map_or("us-central1".to_string(), |r| r.to_string());

            stop_gcp_validator(project_id, &region).await
        }
        _ => Err(anyhow!("Invalid deployment target. Use 'local' or 'gcp'"))
    }
}

// Update the stop_gcp_validator function signature
async fn stop_gcp_validator(project_id: &str, region: &str) -> Result<()> {
    println!("  {} Managing GCP validator...", "→".bold().blue());

    // Get instance details with separate fields
    let describe_output = ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "describe", "arch-validator",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--format", "get(status)"
        ])
        .output()
        .context("Failed to get GCP instance details")?;

    let status = String::from_utf8_lossy(&describe_output.stdout).trim().to_string();
    let zone = format!("{}-a", region);

    if describe_output.status.success() {
        let options = vec!["Suspend instance", "Delete instance"];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do with the GCP validator?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                if status == "SUSPENDED" {
                    println!("  {} Instance is already suspended", "ℹ".bold().blue());
                    return Ok(());
                }

                println!("  {} Suspending GCP validator...", "→".bold().blue());
                let suspend_output = ShellCommand::new("gcloud")
                    .args([
                        "compute", "instances", "suspend",
                        "arch-validator",
                        "--project", project_id,
                        "--zone", &zone,
                        "--quiet"
                    ])
                    .output()
                    .context("Failed to suspend GCP instance")?;

                if !suspend_output.status.success() {
                    return Err(anyhow!(
                        "Failed to suspend GCP instance: {}",
                        String::from_utf8_lossy(&suspend_output.stderr)
                    ));
                }

                println!("{}", "GCP validator suspended successfully!".bold().green());
            }
            1 => {
                let proceed = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Are you sure you want to delete the GCP validator instance? This action cannot be undone.")
                    .default(false)
                    .interact()?;

                if !proceed {
                    println!("  {} Operation cancelled", "ℹ".bold().blue());
                    return Ok(());
                }

                // Delete proxy instance first
                println!("  {} Deleting HTTPS proxy...", "→".bold().blue());
                let _ = ShellCommand::new("gcloud")
                    .args([
                        "compute", "instances", "delete", "arch-validator-proxy",
                        "--project", project_id,
                        "--zone", &zone,
                        "--quiet"
                    ])
                    .output();

                println!("  {} Deleting GCP validator...", "→".bold().blue());
                let delete_output = ShellCommand::new("gcloud")
                    .args([
                        "compute", "instances", "delete",
                        "arch-validator",
                        "--project", project_id,
                        "--zone", &zone,
                        "--quiet"
                    ])
                    .output()
                    .context("Failed to delete GCP instance")?;

                if !delete_output.status.success() {
                    return Err(anyhow!(
                        "Failed to delete GCP instance: {}",
                        String::from_utf8_lossy(&delete_output.stderr)
                    ));
                }

                println!("{}", "GCP validator deleted successfully!".bold().green());
            }
            _ => unreachable!()
        }

        Ok(())
    } else {
        Err(anyhow!("Failed to find GCP validator instance in zone {}", zone))
    }
}

fn stop_local_validator() -> Result<()> {
    println!("  {} Stopping local validator...", "→".bold().blue());

    // Stop the container
    let stop_output = ShellCommand::new("docker")
        .arg("stop")
        .arg("local_validator")
        .output()
        .context("Failed to stop the local validator")?;

    if !stop_output.status.success() {
        println!(
            "  {} Warning: Failed to stop the local validator: {}",
            "⚠".bold().yellow(),
            String::from_utf8_lossy(&stop_output.stderr)
        );
    } else {
        println!("  {} Local validator stopped", "✓".bold().green());
    }

    // Remove the container and its volumes
    let remove_output = ShellCommand::new("docker")
        .arg("rm")
        .arg("-v")  // -v flag removes volumes associated with the container
        .arg("local_validator")
        .output()
        .context("Failed to remove the local validator container")?;

    if !remove_output.status.success() {
        return Err(anyhow!(
            "Failed to remove the local validator container: {}",
            String::from_utf8_lossy(&remove_output.stderr)
        ));
    }

    println!("{}", "Local validator stopped and removed successfully!".bold().green());
    Ok(())
}

pub async fn project_create(args: &CreateProjectArgs, config: &Config) -> Result<()> {
    ensure_global_config()?;
    println!("{}", "Creating a new project...".bold().green());
    
    // Get the project directory from the config or prompt the user
    let project_dir = match config.get_string("project.directory") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => {
            let default_dir = get_default_project_dir();
            prompt_for_project_dir(&default_dir)?
        }
    };

    // Ensure the project directory and its projects subfolder exist
    let projects_subfolder = project_dir.join("projects");
    if !project_dir.exists() {
        fs::create_dir_all(&project_dir).context(format!(
            "Failed to create project directory: {:?}",
            project_dir
        ))?;
        println!(
            "  {} Created project directory at {:?}",
            "✓".bold().green(),
            project_dir
        );
    }
    if !projects_subfolder.exists() {
        fs::create_dir_all(&projects_subfolder).context(format!(
            "Failed to create projects subfolder within project directory: {:?}",
            projects_subfolder
        ))?;
        println!(
            "  {} Created projects subfolder within project directory at {:?}",
            "✓".bold().green(),
            projects_subfolder
        );
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

    // Check if the project already exists
    let new_project_dir = projects_subfolder.join(&project_name);
    if new_project_dir.exists() {
        println!(
            "{}",
            format!("A project named '{}' already exists.", project_name).bold().red()
        );
        return Err(anyhow!("Project already exists"));
    }

    // Create the new project folder
    fs::create_dir_all(&new_project_dir).context(format!(
        "Failed to create project directory: {:?}",
        new_project_dir
    ))?;
    println!(
        "  {} Created project directory at {:?}",
        "✓".bold().green(),
        new_project_dir
    );

    // Create the app folder
    let app_dir = new_project_dir.join("app");
    fs::create_dir_all(&app_dir)
        .context(format!("Failed to create app directory: {:?}", app_dir))?;

    // Copy the sample project files to the new project directory
    extract_project_files(&SAMPLE_DIR, &new_project_dir)?;

    // Create Vite app using npm
    println!("Creating Vite application...");
    let create_vite_output = std::process::Command::new("npm")
        .args(["create", "vite@latest", "frontend", "--", "--template", "react"])
        .current_dir(&app_dir)
        .output()
        .context("Failed to create Vite application")?;

    if !create_vite_output.status.success() {
        return Err(anyhow!("Failed to create Vite application: {}",
            String::from_utf8_lossy(&create_vite_output.stderr)));
    }
    println!("  {} Created Vite application", "✓".bold().green());

    // Change to frontend directory and install base dependencies
    let frontend_dir = app_dir.join("frontend");
    let install_output = std::process::Command::new("npm")
        .arg("install")
        .current_dir(&frontend_dir)
        .output()
        .context("Failed to install base dependencies")?;

    if !install_output.status.success() {
        return Err(anyhow!("Failed to install base dependencies: {}",
            String::from_utf8_lossy(&install_output.stderr)));
    }
    println!("  {} Installed base dependencies", "✓".bold().green());

    // Install additional packages
    let additional_packages = ["sats-connect", "@saturnbtcio/arch-sdk"];
    let install_additional_output = std::process::Command::new("npm")
        .arg("install")
        .args(&additional_packages)
        .current_dir(&frontend_dir)
        .output()
        .context("Failed to install additional packages")?;

    if !install_additional_output.status.success() {
        return Err(anyhow!("Failed to install additional packages: {}",
            String::from_utf8_lossy(&install_additional_output.stderr)));
    }
    println!("  {} Installed additional packages", "✓".bold().green());

    println!("{}", "New project created successfully! 🎉".bold().green());
    println!(
        "  {} Project location: {:?}",
        "ℹ".bold().blue(),
        new_project_dir
    );

    println!("\n{}", "Next steps:".bold().yellow());
    println!(
        "  1. Navigate to {} to find the Rust program template",
        new_project_dir.join("app").join("program").display().to_string().yellow()
    );
    println!("  2. Edit the source code to implement your program logic");
    println!("  3. When ready, run {} to compile and deploy your program to the network", "arch-cli deploy".cyan());
    println!("\n{}", "Need help? Check out our documentation at https://docs.arch.network".italic());

    Ok(())
}

pub async fn project_deploy(config: &Config) -> Result<()> {
    println!("{}", "Deploying a project...".bold().green());

    // Get the project directory from the config
    let project_dir = PathBuf::from(config.get_string("project.directory")?);

    // Get list of projects
    let projects: Vec<_> = fs::read_dir(&project_dir)?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.is_dir() && path.join("program").exists() {
                    Some(path.file_name().unwrap().to_string_lossy().into_owned())
                } else {
                    None
                }
            })
        })
        .collect();

    if projects.is_empty() {
        println!("No deployable projects found. Make sure your projects have a 'program' folder.");
        return Ok(());
    }

    // Ask user to select a project
    let selection = Select::new()
        .with_prompt("Select a project to deploy")
        .items(&projects)
        .interact()?;

    let selected_project = &projects[selection];
    let program_dir = project_dir.join(selected_project).join("app/program");

    println!("Deploying project: {}", selected_project.yellow());

    // Here, call your existing deploy function with the program_dir
    // You may need to modify your existing deploy function to accept a PathBuf instead of DeployArgs
    let rpc_url = "";
    if let Err(e) = deploy_program_from_path(&program_dir, config, None, rpc_url.to_string()).await {
        println!("Failed to deploy program: {}", e);
        return Err(e);
    }

    println!("{}", "Project deployed successfully!".bold().green());
    Ok(())
}

fn ensure_default_config() -> Result<()> {
    let config_path = get_config_path()?;
    if !config_path.exists() {
        let default_config_content = include_str!("../templates/config.default.toml");
        fs::write(&config_path, default_config_content)?;
        println!(
            "  {} Created default configuration at {:?}",
            "✓".bold().green(),
            config_path
        );
    }
    Ok(())
}

pub fn ensure_global_config() -> Result<()> {
    let config_dir = get_config_dir()?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
        println!("Created global configuration directory at {:?}", config_dir);
    }

    ensure_default_config()?;

    // Ensure other necessary directories and files exist
    let arch_data_dir = config_dir.join("arch-data");
    if !arch_data_dir.exists() {
        fs::create_dir_all(&arch_data_dir)?;
        println!("Created arch-data directory at {:?}", arch_data_dir);
    }

    // Copy template files if they don't exist
    copy_template_files()?;

    Ok(())
}

fn copy_template_files() -> Result<()> {
    let config_dir = get_config_dir()?;
    // Files that should always be updated on new versions
    let force_update_templates = [
        "server-docker-compose.yml",
    ];

    let templates = [
        ("config.default.toml", "config.toml"),
        ("init.sh", "init.sh"),
        ("bootnode.sh", "bootnode.sh"),
        ("arch-docker-compose.yml", "arch-docker-compose.yml"),
        ("bitcoin-docker-compose.yml", "bitcoin-docker-compose.yml"),
        ("btc-rpc-explorer.dockerfile", "btc-rpc-explorer.dockerfile"),
        ("server-docker-compose.yml", "server-docker-compose.yml"),
        ("leader.sh", "leader.sh"),
        ("validator.sh", "validator.sh"),
    ];

    for (template, dest) in templates.iter() {
        let dest_path = config_dir.join(dest);
        let should_update = !dest_path.exists() || force_update_templates.contains(dest);

        if should_update {
            let template_content = match *template {
                "config.default.toml" => include_str!("../templates/config.default.toml"),
                "init.sh" => include_str!("../templates/init.sh"),
                "bootnode.sh" => include_str!("../templates/bootnode.sh"),
                "arch-docker-compose.yml" => include_str!("../templates/arch-docker-compose.yml"),
                "bitcoin-docker-compose.yml" => include_str!("../templates/bitcoin-docker-compose.yml"),
                "btc-rpc-explorer.dockerfile" => include_str!("../templates/btc-rpc-explorer.dockerfile"),
                "server-docker-compose.yml" => include_str!("../templates/server-docker-compose.yml"),
                "leader.sh" => include_str!("../templates/leader.sh"),
                "validator.sh" => include_str!("../templates/validator.sh"),
                _ => return Err(anyhow!("Unknown template file: {}", template)),
            };

            if dest_path.exists() {
                println!("Updating {} with new version", dest);
            } else {
                println!("Creating {} at {:?}", dest, dest_path);
            }

            fs::write(&dest_path, template_content)?;
        }
    }

    Ok(())
}

// Add after the start_gcp_validator function
async fn setup_ssl_proxy(project_id: &str, region: &str, validator_ip: &str) -> Result<()> {
    println!("  {} Setting up HTTPS proxy...", "→".bold().blue());

    // Create a temporary directory for the build
    let temp_dir = tempfile::tempdir()?;

    // Create nginx.conf
    let nginx_conf = format!(r#"
events {{
    worker_connections 1024;
}}
http {{
    # Error log configuration
    error_log /dev/stderr debug;
    access_log /dev/stdout combined;

    # Longer timeouts
    proxy_connect_timeout 60;
    proxy_send_timeout 60;
    proxy_read_timeout 60;

    server {{
        listen 443 ssl;
        server_name _;
        
        ssl_certificate /etc/nginx/ssl/nginx.crt;
        ssl_certificate_key /etc/nginx/ssl/nginx.key;
        
        location / {{
            proxy_pass http://{}:9001;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;

            # Debug headers
            add_header X-Debug-Target "http://{}:9001" always;
            add_header X-Debug-Host $host always;
        }}
    }}
}}
"#, validator_ip, validator_ip);

    fs::write(temp_dir.path().join("nginx.conf"), nginx_conf)?;

    // Create Dockerfile for SSL proxy
    let dockerfile_content = r#"FROM --platform=linux/amd64 nginx:alpine
COPY nginx.conf /etc/nginx/nginx.conf
RUN mkdir -p /etc/nginx/ssl
RUN apk add --no-cache openssl
RUN openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout /etc/nginx/ssl/nginx.key \
    -out /etc/nginx/ssl/nginx.crt \
    -subj "/CN=arch-validator/O=Arch Network/C=US"
EXPOSE 443
"#;

    fs::write(temp_dir.path().join("Dockerfile"), dockerfile_content)?;

    // Create and push the proxy image
    let proxy_image = format!("gcr.io/{}/arch-validator-proxy:latest", project_id);

    println!("  {} Building and pushing proxy image...", "→".bold().blue());
    let build_status = Command::new("docker")
        .args([
            "build",
            "-t", &proxy_image,
            temp_dir.path().to_str().unwrap(),
        ])
        .status()
        .context("Failed to build proxy image")?;

    if !build_status.success() {
        return Err(anyhow!("Failed to build proxy image"));
    }

    let push_status = Command::new("docker")
        .args(["push", &proxy_image])
        .status()
        .context("Failed to push proxy image")?;

    if !push_status.success() {
        return Err(anyhow!("Failed to push proxy image"));
    }

    // Create firewall rule for internal communication
    println!("  {} Creating firewall rule for internal communication...", "→".bold().blue());
    let _ = ShellCommand::new("gcloud")
        .args([
            "compute", "firewall-rules", "create", "allow-validator-internal",
            "--project", project_id,
            "--allow", "tcp:9001",
            "--source-tags", "validator-proxy",
            "--target-tags", "validator",
            "--description", "Allow proxy to validator communication",
        ])
        .output();

    // Create firewall rule for HTTPS
    println!("  {} Creating firewall rule for HTTPS...", "→".bold().blue());
    let _ = ShellCommand::new("gcloud")
        .args([
            "compute", "firewall-rules", "create", "allow-validator-https",
            "--project", project_id,
            "--allow", "tcp:443",
            "--target-tags", "validator-proxy",
            "--description", "Allow incoming HTTPS traffic for validator proxy",
        ])
        .output();

    // Deploy the proxy container
    println!("  {} Deploying HTTPS proxy...", "→".bold().blue());
    let create_proxy_output = ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "create-with-container", "arch-validator-proxy",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--machine-type", "e2-micro",
            "--container-image", &proxy_image,
            "--tags", "validator-proxy",
            // "--platform", "linux/amd64",
        ])
        .output()
        .context("Failed to create proxy instance")?;

    if !create_proxy_output.status.success() {
        return Err(anyhow!(
            "Failed to create proxy instance: {}",
            String::from_utf8_lossy(&create_proxy_output.stderr)
        ));
    }

    // Get the proxy's external IP
    let proxy_ip = String::from_utf8_lossy(&ShellCommand::new("gcloud")
        .args([
            "compute", "instances", "describe", "arch-validator-proxy",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--format", "get(networkInterfaces[0].accessConfigs[0].natIP)"
        ])
        .output()?
        .stdout).trim().to_string();

    // Add after getting the proxy's external IP
    println!("\n{}", "Running connectivity tests...".bold().blue());

    // Test if validator is reachable from proxy
    let test_connection = ShellCommand::new("gcloud")
        .args([
            "compute", "ssh", "arch-validator-proxy",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--command", &format!("curl -v http://{}:9001", validator_ip)
        ])
        .output()
        .context("Failed to test connection")?;

    println!("Connection test result:");
    println!("{}", String::from_utf8_lossy(&test_connection.stdout));
    println!("{}", String::from_utf8_lossy(&test_connection.stderr));

    // Check nginx logs
    let check_logs = ShellCommand::new("gcloud")
        .args([
            "compute", "ssh", "arch-validator-proxy",
            "--project", project_id,
            "--zone", &format!("{}-a", region),
            "--command", "docker logs $(docker ps -q)"
        ])
        .output()
        .context("Failed to check nginx logs")?;

    println!("\nNginx logs:");
    println!("{}", String::from_utf8_lossy(&check_logs.stdout));
    println!("{}", String::from_utf8_lossy(&check_logs.stderr));

    println!("\n{}", "HTTPS proxy setup complete!".bold().green());
    println!("Proxy IP: {}", proxy_ip);
    println!("HTTPS endpoint: {}", format!("https://{}", proxy_ip).yellow());
    println!("\nNote: Using self-signed certificate. You may need to accept the security warning in your browser.");

    Ok(())
}

pub async fn assign_ownership(args: &AssignOwnershipArgs, config: &Config) -> Result<()> {
    println!("{}", "Assigning program ownership...".bold().green());

    // Get the keys file
    let keys_file = get_config_dir()?.join("keys.json");

    // Get the keypair and pubkey for the account
    let (caller_keypair, caller_pubkey) = if args.identifier.len() == 64 {
        // If identifier is a public key
        let key_name = find_key_name_by_pubkey(&keys_file, &args.identifier)?;
        let pubkey_bytes = hex::decode(&args.identifier)?;
        (
            get_keypair_from_name(&key_name, &keys_file)?,
            Pubkey::from_slice(&pubkey_bytes),
        )
    } else {
        // If identifier is a name
        let pubkey = get_pubkey_from_name(&args.identifier, &keys_file)?;
        let pubkey_bytes = hex::decode(&pubkey)?;
        (
            get_keypair_from_name(&args.identifier, &keys_file)?,
            Pubkey::from_slice(&pubkey_bytes),
        )
    };

    // Decode program ID
    let program_id_bytes = hex::decode(&args.program_id)
        .context("Failed to decode program ID from hex")?;
    let program_id = Pubkey::from_slice(&program_id_bytes);

    // Get RPC URL
    let rpc_url = get_rpc_url_with_fallback(args.rpc_url.clone(), config).unwrap();
    println!("  {} RPC URL: {}", "ℹ".bold().blue(), rpc_url.yellow());

    // Transfer ownership
    transfer_account_ownership(
        &caller_keypair,
        &caller_pubkey,
        &program_id,
        rpc_url,
    ).await?;

    println!(
        "  {} Successfully transferred ownership to program: {}",
        "✓".bold().green(),
        args.program_id.bright_green()
    );

    Ok(())
}

pub async fn update_account(args: &UpdateAccountArgs, config: &Config) -> Result<()> {
    println!("{}", "Updating account data...".bold().green());

    // Get the keys file
    let keys_file = get_config_dir()?.join("keys.json");

    // Get the keypair and pubkey for the account
    let (caller_keypair, caller_pubkey) = if args.identifier.len() == 64 {
        // If identifier is a public key
        let key_name = find_key_name_by_pubkey(&keys_file, &args.identifier)?;
        let pubkey_bytes = hex::decode(&args.identifier)?;
        let pubkey = Pubkey::from_slice(&pubkey_bytes);
        (
            get_keypair_from_name(&key_name, &keys_file)?,
            pubkey,
        )
    } else {
        // If identifier is a name
        let pubkey = get_pubkey_from_name(&args.identifier, &keys_file)?;
        let pubkey_bytes = hex::decode(&pubkey)?;
        let pubkey = Pubkey::from_slice(&pubkey_bytes);
        (
            get_keypair_from_name(&args.identifier, &keys_file)?,
            pubkey,
        )
    };

    // Read the data file
    let data = fs::read(&args.data_file)
        .context(format!("Failed to read data file: {:?}", args.data_file))?;

    // Get RPC URL
    let rpc_url = get_rpc_url_with_fallback(args.rpc_url.clone(), config).unwrap();
    println!("  {} RPC URL: {}", "ℹ".bold().blue(), rpc_url.yellow());

    // Create the extend bytes instruction
    let caller_keypair_clone = caller_keypair.clone();
    let caller_pubkey_clone = caller_pubkey;
    let rpc_url_clone = rpc_url.clone();
    let data_clone = data.clone();

    // Send the extend bytes instruction
    let (txid, _) = tokio::task::spawn_blocking(move || {
        sign_and_send_instruction(
            SystemInstruction::new_extend_bytes_instruction(
                data_clone,
                caller_pubkey_clone,
            ),
            vec![caller_keypair_clone],
            rpc_url_clone,
        )
    }).await??;

    println!(
        "  {} Successfully updated account data. Transaction ID: {}",
        "✓".bold().green(),
        txid.yellow()
    );
    println!(
        "  {} Updated {} bytes",
        "ℹ".bold().blue(),
        data.len().to_string().bright_white()
    );

    Ok(())
}

pub fn load_and_update_config(config_path: &str) -> Result<Config> {
    let config_file_path = Path::new(config_path);

    if config_file_path.exists() {
        // Existing logic to load and update the config
        let mut config_content = fs::read_to_string(config_file_path)
            .context("Failed to read existing config.toml")?;
        let mut doc = config_content.parse::<Document>()
            .context("Failed to parse existing config.toml")?;
    
        // Check if e2e network is already present
        if !doc["networks"]["e2e"].is_table() {
            // Add e2e network configuration
            doc["networks"]["e2e"] = Item::Table({
                let mut table = toml_edit::Table::new();
                table["type"] = value("e2e");
                table["bitcoin_rpc_endpoint"] = value("localhost");
                table["bitcoin_rpc_port"] = value("18443");
                table["bitcoin_rpc_user"] = value("bitcoin");
                table["bitcoin_rpc_password"] = value("password");
                table["bitcoin_rpc_wallet"] = value("devwallet");
                table["docker_compose_file"] = value("./server-docker-compose.yml");
                table["leader_rpc_endpoint"] = value("http://localhost:9002");
                let mut services = toml_edit::Array::new();
                services.push("bitcoin");
                services.push("electrs");
                services.push("btc-rpc-explorer");
                services.push("local_validator");
                table["services"] = value(services);
                table
            });
    
            // Save the updated config back to the file
            fs::write(config_file_path, doc.to_string())
                .context("Failed to write updated config.toml")?;
        }
    } else {
        // If config.toml does not exist, create it from the default template
        ensure_default_config()?;
    }
    
    // Load the configuration using the existing method
    load_config(config_path)
}

fn find_program_so_file(path: &PathBuf) -> Result<PathBuf> {
    if path.extension().map_or(false, |ext| ext == "so") {
        // If path directly points to .so file
        Ok(path.clone())
    } else {
        // Look for .so file in release directory
        let release_dir = path.join("target/sbf-solana-solana/release");
        fs::read_dir(&release_dir)
            .with_context(|| format!("Failed to read release directory at {:?}", release_dir))?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry.path().extension()
                    .map_or(false, |ext| ext == "so")
            })
            .map(|entry| entry.path())
            .ok_or_else(|| anyhow!("No .so file found in release directory"))
    }
}
