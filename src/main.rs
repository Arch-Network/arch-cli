use arch_cli::*;
use anyhow::Result;
use config::{ Config, File, Environment };
use colored::*;
use dotenv::dotenv;
use clap::Parser;
use anyhow::Context;


#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    println!("{}", "Welcome to the Arch Network CLI".bold().green());

    // Parse command-line arguments
    let cli = Cli::parse();

    // Load configuration
    let config = Config::builder()
        .add_source(File::with_name("config.toml"))
        .add_source(Environment::default())
        .build()
        .context("Failed to load configuration")?;

    // Match on the subcommand
    match &cli.command {
        Commands::Init => init().await?,
        Commands::StartServer => start_server(&config).await?,
        Commands::Deploy(args) => deploy(&args, &config).await?,
        Commands::StopServer => stop_server().await?,
        Commands::Clean => clean().await?,
        Commands::StartDkg => start_dkg(&config).await?,
        Commands::SendCoins(args) => send_coins(&args, &config).await?,
        Commands::StartApp => start_app().await?,
    }

    Ok(())
}