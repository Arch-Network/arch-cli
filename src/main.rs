use clap::{ Parser, Subcommand, Command };
use std::fs;
use tokio;
use anyhow::{ Context, Result };
use std::process::Command as ShellCommand;

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
    Deploy {
        example_name: String,
    },
    StopServer,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => init(),
        Commands::StartServer => start_server(),
        Commands::Deploy { example_name } => deploy(example_name),
        Commands::StopServer => stop_server(),
    }
}

fn init() -> Result<()> {
    println!("Initializing new arch-bitcoin app...");

    // Create project structure
    let dirs = ["src/app/program/src", "src/app/backend", "src/app/frontend"];

    for dir in dirs.iter() {
        fs::create_dir_all(dir).with_context(|| format!("Failed to create directory: {}", dir))?;
    }

    // Create boilerplate files
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

    println!("New arch-bitcoin app initialized successfully!");
    Ok(())
}
fn start_server() -> Result<()> {
    println!("Starting development server...");

    ShellCommand::new("sh").arg("-c").arg("./start-server.sh 3").spawn()?;
    println!("Development server started successfully!");

    Ok(())
}
fn deploy(example_name: &str) -> Result<()> {
    println!("Deploying example: {}", example_name);
    // Build the program
    ShellCommand::new("cargo")
        .args(&["build-sbf", "--manifest-path", &format!("examples/{}/Cargo.toml", example_name)])
        .status()?;
    // Deploy the program (you'll need to implement this part based on your deployment process)
    // For example:
    // ShellCommand::new("arch-deploy")
    //     .arg(&format!("target/sbf-solana-solana/release/{}.so", example_name))
    //     .status()?;
    println!("Example {} deployed successfully!", example_name);
    Ok(())
}

fn stop_server() -> Result<()> {
    println!("Stopping development server...");
    ShellCommand::new("pkill").arg("-f").arg("start-server.sh").status()?;
    println!("Development server stopped successfully!");
    Ok(())
}
