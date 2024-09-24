use arch_cli::*;
use anyhow::Result;
use config::{Config, File, Environment};
use colored::*;
use dotenv::dotenv;
use clap::Parser;
use anyhow::Context;
use std::env;
use std::fs;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    println!("{}", "Welcome to the Arch Network CLI".bold().green());

    // Parse command-line arguments
    let cli = Cli::parse();

    // Load configuration
    let config = load_config()?;

    // Set verbose mode if flag is present
    if cli.verbose {
        // Set up verbose logging or output here
    }

    // Match on the subcommand
    let result = match &cli.command {
        Commands::Init => init().await,
        Commands::Server(ServerCommands::Start) => server_start(&config).await,
        Commands::Server(ServerCommands::Stop) => server_stop().await,
        Commands::Server(ServerCommands::Status) => server_status(&config).await,
        Commands::Server(ServerCommands::Logs { service }) => server_logs(service, &config).await,
        Commands::Deploy(args) => deploy(args, &config).await,
        Commands::Project(ProjectCommands::Clean) => clean().await,
        Commands::Dkg(DkgCommands::Start) => start_dkg(&config).await,
        Commands::Bitcoin(BitcoinCommands::SendCoins(args)) => send_coins(args, &config).await,
        Commands::Frontend(FrontendCommands::Start) => frontend_start().await,
        Commands::Account(AccountCommands::Create(args)) => create_account(args, &config).await,
        Commands::Account(AccountCommands::List) => list_accounts().await,
        Commands::Account(AccountCommands::Delete(args)) => delete_account(args).await,
        Commands::Config(ConfigCommands::View) => config_view(&config).await,
        Commands::Config(ConfigCommands::Edit) => config_edit().await,
        Commands::Config(ConfigCommands::Reset) => config_reset().await,
        Commands::Start => server_start(&config).await,
        Commands::Stop => server_stop().await,
    };

    if let Err(e) = result {
        println!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}