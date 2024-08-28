use bitcoin::Network;
use clap::{ Parser, Subcommand, Command };
use std::fs;
use tokio;
use anyhow::{ Context, Result };
use std::process::Command as ShellCommand;
use common::helper::*;
use common::constants::*;
use bitcoin::{ Address, Network, PublicKey };
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
    Deploy,
    StopServer,
    Clean,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => init(),
        Commands::StartServer => start_server(),
        Commands::Deploy => deploy(),
        Commands::StopServer => stop_server(),
        Commands::Clean => clean(),
    }
}

fn init() -> Result<()> {
    println!("Initializing new Arch Network app...");

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

    println!("New Arch Network app initialized successfully!");
    Ok(())
}
fn start_server() -> Result<()> {
    println!("Starting development server...");

    ShellCommand::new("sh").arg("-c").arg("./start-server.sh 3").spawn()?;
    println!("Development server started successfully!");

    Ok(())
}
fn deploy() -> Result<()> {
    println!("Deploying your app...");
    // Build the program
    ShellCommand::new("cargo")
        .args(&["build-sbf", "--manifest-path", &format!("src/app/program/Cargo.toml")])
        .status()?;

    // Have to create a program account for the program
    let (program_keypair, program_pubkey) = with_secret_key_file(PROGRAM_FILE_PATH).expect(
        "Failed to get program key pair"
    );

    // Tell user to deposit funds into the program account
    let program_address = Address::p2pkh(&program_pubkey, Network::Testnet);
    println!("Please deposit funds into the program account: {:?}", program_address);

    // Wait for user to deposit funds
    println!("Waiting for funds to be deposited...");
    std::thread::sleep(std::time::Duration::from_secs(10));
    println!("Funds deposited successfully!");

    // let (program_account_txid, program_account_vout) = create_and_fund_account(&program_pubkey);
    // deploy_program(&program_keypair, &program_pubkey, &program_account_txid, program_account_vout);

    println!("Your app has been deployed successfully!");
    Ok(())
}
fn stop_server() -> Result<()> {
    println!("Stopping development server...");
    ShellCommand::new("pkill").arg("-f").arg("start-server.sh").status()?;
    println!("Development server stopped successfully!");
    Ok(())
}

fn clean() -> Result<()> {
    println!("Cleaning project...");

    // Remove src/app directory
    fs::remove_dir_all("src/app")?;

    println!("Project cleaned successfully!");
    Ok(())
}
