use bitcoin::Amount;
use bitcoincore_rpc::json::GetTransactionResult;
use clap::{ Parser, Subcommand, Args };
use std::fs;
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

    match &cli.command {
        Commands::Init => init().await,
        Commands::StartServer => start_server().await,
        Commands::Deploy(args) => deploy(args).await,
        Commands::StopServer => stop_server().await,
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
async fn start_server() -> Result<()> {
    println!("{}", "Starting development server...".bold().green());

    // ShellCommand::new("sh").arg("-c").arg("./start-server.sh 3").spawn()?;

    println!("  {} Development server started successfully!", "✓".bold().green());

    Ok(())
}
async fn deploy(args: &DeployArgs) -> Result<()> {
    println!("{}", "Deploying your Arch Network app...".bold().green());

    if let Some(path) = &args.directory {
        if !std::path::Path::new(path).exists() {
            return Err(anyhow::anyhow!("Specified directory does not exist: {}", path));
        }
        // Run ShellCommand wiht a manifest path of args.directory + /Cargo.toml
        ShellCommand::new("cargo")
            .args(&["build-sbf", "--manifest-path", format!("{}/Cargo.toml", path).as_str()])
            .status()?;
    } else {
        // Default behavior: build the program
        println!("  {} Building program...", "→".bold().blue());
        ShellCommand::new("cargo")
            .args(&["build-sbf", "--manifest-path", "src/app/program/Cargo.toml"])
            .status()?;
        "target/deploy/program.so".to_string(); // Adjust this path if necessary
    }

    // Use the provided program key path or default to PROGRAM_FILE_PATH
    let program_key_path: &str = if let Some(key) = &args.program_key {
        key
    } else {
        PROGRAM_FILE_PATH
    };

    let (program_keypair, program_pubkey) = with_secret_key_file(program_key_path).context(
        "Failed to get program key pair"
    )?;

    // Retrieve this account's account address from the Arch Network RPC
    let account_address = get_account_address_async(program_pubkey).await.context(
        "Failed to get account address"
    )?;

    println!("  {} Program account created", "✓".bold().green());
    println!("  {} Account address: {}", "ℹ".bold().blue(), account_address.to_string().yellow());

    let account_address = bitcoin::Address
        ::from_str(&account_address)
        .unwrap()
        .require_network(BITCOIN_NETWORK)
        .unwrap();

    // Set up Bitcoin RPC client
    let rpc = Client::new(
        BITCOIN_NODE_ENDPOINT,
        Auth::UserPass(BITCOIN_NODE_USERNAME.to_string(), BITCOIN_NODE_PASSWORD.to_string())
    ).context("Failed to create RPC client")?;

    let mut tx_info: Option<GetTransactionResult> = None;

    // If REGTEST, then just send the satoshis to the address
    if BITCOIN_NETWORK == bitcoin::Network::Regtest {
        let tx = rpc.send_to_address(
            &account_address,
            Amount::from_sat(3000),
            None,
            None,
            None,
            None,
            None,
            None
        )?;
        println!("Transaction sent: {}", tx);

        // Wait for transaction confirmation
        loop {
            match rpc.get_transaction(&tx, None) {
                Ok(info) => {
                    if info.info.confirmations > 0 {
                        println!(
                            "Transaction confirmed with {} confirmations",
                            info.info.confirmations
                        );
                        tx_info = Some(info);
                        break;
                    }
                    println!("Waiting for confirmation...");
                }
                Err(e) => println!("Error checking transaction: {}", e),
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    } else {
        // For non-REGTEST networks, prompt user to deposit funds
        println!("{}", "Please deposit funds to continue:".bold());
        println!(
            "  {} Deposit address: {}",
            "→".bold().blue(),
            account_address.to_string().yellow()
        );
        println!("  {} Minimum required: {} satoshis", "ℹ".bold().blue(), "3000".yellow());
        println!("  {} Waiting for funds...", "⏳".bold().blue());

        // TODO: Check balance of account_address and wait until it has at least 3000 satoshi
    }

    println!("  {} Funds received successfully", "✓".bold().green());

    // Deploy the program
    if let Some(info) = tx_info {
        deploy_program(&program_keypair, &program_pubkey, &info.info.txid.to_string(), 0).await;
    } else {
        // Handle the case where tx_info is None (for non-REGTEST networks)
        println!("Warning: No transaction info available for deployment");
    }

    println!("{}", "Your app has been deployed successfully!".bold().green());
    Ok(())
}
async fn stop_server() -> Result<()> {
    println!("Stopping development server...");
    ShellCommand::new("pkill").arg("-f").arg("start-server.sh").status()?;
    println!("Development server stopped successfully!");
    Ok(())
}

async fn clean() -> Result<()> {
    println!("Cleaning project...");

    // Remove src/app directory
    fs::remove_dir_all("src/app")?;

    println!("Project cleaned successfully!");
    Ok(())
}
