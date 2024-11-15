use crate::{
    build_frontend, create_account, deploy_program_from_path, extract_recursive,
    find_key_name_by_pubkey, get_config_dir, get_keypair_from_name, get_pubkey_from_name,
    key_name_exists, make_program_executable, setup_base_structure, Config, CreateAccountArgs,
    DemoStartArgs, PROJECT_DIR,
};
use anyhow::{Context, Result};
use arch_program::pubkey::Pubkey;
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};

pub async fn setup_demo_environment(
    args: &DemoStartArgs,
    config: &Config,
) -> Result<(PathBuf, String, String, String)> {
    println!("{}", "Setting up demo environment...".bold().green());

    // Get network type from config
    let network = config
        .get_string("bitcoin.network")
        .unwrap_or_else(|_| "regtest".to_string());
    println!("Network type: {}", network);

    let rpc_url = get_rpc_url_with_fallback(args.rpc_url.clone(), config).unwrap();
    println!("Using RPC URL: {}", rpc_url);

    // Set up base structure
    let (_, projects_dir) = setup_base_structure(config)?;

    // Set up demo project
    let demo_dir = projects_dir.join("demo");
    if !demo_dir.exists() {
        println!(
            "  {} Demo directory not found. Creating it...",
            "ℹ".bold().blue()
        );
        fs::create_dir_all(&demo_dir)?;

        // Extract demo-specific files
        let demo_files_dir = PROJECT_DIR.get_dir("app").unwrap();
        extract_recursive(&demo_files_dir, &demo_dir.join("app"))?;

        // Handle .env file
        let env_example_file = demo_dir.join("app/frontend/.env.example");
        if env_example_file.exists() {
            fs::rename(&env_example_file, demo_dir.join("app/frontend/.env"))?;
        }
    }

    // Change to the demo directory
    std::env::set_current_dir(&demo_dir).context("Failed to change to demo directory")?;

    let env_file = demo_dir.join("app/frontend/.env");
    println!(
        "  {} Reading .env file from: {:?}",
        "ℹ".bold().blue(),
        env_file
    );

    // Read or create .env file
    let env_content = fs::read_to_string(&env_file).context("Failed to read .env file")?;

    // Get or create program pubkey
    let mut program_pubkey = env_content
        .lines()
        .find_map(|line| line.strip_prefix("VITE_PROGRAM_PUBKEY="))
        .unwrap_or("")
        .to_string();

    let keys_file = get_config_dir()?.join("keys.json");
    let graffiti_key_name: String;

    if program_pubkey.is_empty() {
        // Create new program account
        graffiti_key_name = create_unique_key_name(&keys_file)?;

        println!("Creating account with name: {}", graffiti_key_name);
        create_account(
            &CreateAccountArgs {
                name: graffiti_key_name.clone(),
                program_id: None,
                rpc_url: Some(rpc_url.clone()),
            },
            config,
        )
        .await?;

        program_pubkey = get_pubkey_from_name(&graffiti_key_name, &keys_file)?;
    } else {
        graffiti_key_name = find_key_name_by_pubkey(&keys_file, &program_pubkey)?;
        println!("Using existing account with name: {}", graffiti_key_name);
    }

    // Deploy program
    let program_keypair = get_keypair_from_name(&graffiti_key_name, &keys_file)?;
    let program_pubkey_bytes = Pubkey::from_slice(&program_keypair.public_key().serialize()[1..33]);

    // Note: Using shared program directory for deployment
    // Clone the rpc_url before using it to avoid the "use of moved value" error
    let rpc_url_clone = rpc_url.clone();
    deploy_program_from_path(
        &projects_dir.join("program"), // Using shared program directory
        config,
        Some((program_keypair.clone(), program_pubkey_bytes)),
        rpc_url_clone,
    )
    .await?;

    // Clone the rpc_url before using it to avoid the "use of moved value" error
    let rpc_url_clone = rpc_url.clone();
    make_program_executable(&program_keypair, &program_pubkey_bytes, rpc_url_clone).await?;

    // Setup wall account
    let wall_pubkey = if key_name_exists(&keys_file, "graffiti_wall_state")? {
        println!(
            "  {} Using existing graffiti_wall_state account",
            "ℹ".bold().blue()
        );
        get_pubkey_from_name("graffiti_wall_state", &keys_file)?
    } else {
        println!(
            "  {} Creating new graffiti_wall_state account",
            "ℹ".bold().blue()
        );
        create_account(
            &CreateAccountArgs {
                name: "graffiti_wall_state".to_string(),
                program_id: Some(hex::encode(program_pubkey_bytes.serialize())),
                rpc_url: Some(rpc_url.clone()),
            },
            config,
        )
        .await?;
        get_pubkey_from_name("graffiti_wall_state", &keys_file)?
    };

    build_frontend(
        &demo_dir,
        Some(&rpc_url),
        &program_pubkey,
        &wall_pubkey,
        &network,
    )?;

    Ok((demo_dir, program_pubkey, wall_pubkey, rpc_url))
}

fn create_unique_key_name(keys_file: &PathBuf) -> Result<String> {
    let mut name = String::from("graffiti");
    let mut counter = 1;
    while key_name_exists(keys_file, &name)? {
        name = format!("graffiti_{}", counter);
        counter += 1;
    }
    Ok(name)
}

fn get_rpc_url(args: &DemoStartArgs, config: &Config) -> Result<String> {
    get_rpc_url_with_fallback(args.rpc_url.clone(), config)
}

pub fn get_rpc_url_with_fallback(rpc_url: Option<String>, config: &Config) -> Result<String> {
    println!(
        "config.leader_rpc_endpoint: {}",
        config.get_string("leader_rpc_endpoint").unwrap()
    );
    Ok(rpc_url.unwrap_or_else(|| {
        config
            .get_string("leader_rpc_endpoint")
            .unwrap_or_else(|_| common::constants::NODE1_ADDRESS.to_string())
    }))
}

fn update_demo_cargo_toml(demo_dir: &Path, base_dir: &Path) -> Result<()> {
    let cargo_path = demo_dir.join("Cargo.toml");
    let cargo_content = r#"[package]
name = "arch-demo-app"
version = "0.1.0"
edition = "2021"

[dependencies]
common = { path = "../../common" }
program = { path = "../../program" }
bip322 = { path = "../../bip322" }
"#;

    fs::write(cargo_path, cargo_content)?;
    Ok(())
}
