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

    // Determine the configuration file path
    let config_path = env::var("ARCH_CLI_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut default_path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
            default_path.push("arch-cli");
            default_path.push("config.toml");
            default_path
        });

    // Check if the configuration file exists, if not copy the default template
    if !config_path.exists() {
        println!("Configuration file not found at {:?}. Creating a default configuration file.", config_path);
        copy_default_config(&config_path)?;
    }

    // Load configuration
    let config = Config::builder()
        .add_source(File::from(config_path))
        .add_source(Environment::default())
        .build()
        .context("Failed to load configuration")?;

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
        Commands::Config(ConfigCommands::View) => config_view(&config).await,
        Commands::Config(ConfigCommands::Edit) => config_edit(&config).await,
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

fn copy_default_config(config_path: &PathBuf) -> Result<()> {
    let default_config_path = PathBuf::from("config.default.toml");

    if !default_config_path.exists() {
        return Err(anyhow::anyhow!("Default configuration template not found at {:?}", default_config_path));
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(default_config_path, config_path)?;
    println!("Default configuration file created at {:?}", config_path);
    Ok(())
}