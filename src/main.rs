use anyhow::Result;
use arch_cli::*;
use clap::Parser;
use colored::*;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    println!("{}", "Welcome to the Arch Network CLI".bold().green());

    if let Err(e) = ensure_global_config() {
        eprintln!("Failed to initialize global configuration: {}", e);
        std::process::exit(1);
    }

    // Parse command-line arguments
    let cli = Cli::parse();

    // Load configuration
    let config = load_config(&cli.network)?;

    // Set verbose mode if flag is present
    if cli.verbose {
        // Set up verbose logging or output here
    }

    // Match on the subcommand
    let result = match &cli.command {
        Commands::Init => init().await,
        Commands::Server(ServerCommands::Start) => server_start(&config).await,
        Commands::Server(ServerCommands::Stop) => server_stop(&config).await,
        Commands::Server(ServerCommands::Status) => server_status(&config).await,
        Commands::Server(ServerCommands::Logs { service }) => server_logs(service, &config).await,
        Commands::Server(ServerCommands::Clean) => server_clean(&config).await,
        Commands::Deploy(args) => deploy(args, &config).await,
        Commands::Dkg(DkgCommands::Start) => start_dkg(&config).await,
        Commands::Bitcoin(BitcoinCommands::SendCoins(args)) => send_coins(args, &config).await,
        Commands::Demo(DemoCommands::Start(args)) => demo_start(args, &config).await,
        Commands::Demo(DemoCommands::Stop) => demo_stop(&config).await,
        Commands::Account(AccountCommands::Create(args)) => create_account(args, &config).await,
        Commands::Account(AccountCommands::List) => list_accounts().await,
        Commands::Account(AccountCommands::Delete(args)) => delete_account(args).await,
        Commands::Account(AccountCommands::AssignOwnership(args)) => assign_ownership(args, &config).await,
        Commands::Account(AccountCommands::Update(args)) => update_account(args, &config).await,
        Commands::Config(ConfigCommands::View) => config_view(&config).await,
        Commands::Config(ConfigCommands::Edit) => config_edit().await,
        Commands::Config(ConfigCommands::Reset) => config_reset().await,
        Commands::Start => server_start(&config).await,
        Commands::Stop => server_stop(&config).await,
        Commands::Indexer(IndexerCommands::Start(args)) => indexer_start(args, &config).await,
        Commands::Indexer(IndexerCommands::Stop(args)) => indexer_stop(args, &config).await,
        Commands::Indexer(IndexerCommands::Clean) => indexer_clean(&config).await,
        Commands::Project(ProjectCommands::Create(args)) => create_project(args, &config).await,
        Commands::Project(ProjectCommands::Deploy) => project_deploy(&config).await,
        Commands::Validator(ValidatorCommands::Start(args)) => validator_start(args, &config).await,
        Commands::Validator(ValidatorCommands::Stop(args)) => validator_stop(&args).await,
    };

    if let Err(e) = result {
        println!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
