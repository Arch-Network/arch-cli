use crate::{
    create_account, deploy_program_from_path, extract_project_files, find_key_name_by_pubkey,
    get_config_dir, get_keypair_from_name, get_pubkey_from_name, key_name_exists,
    make_program_executable, Config, CreateAccountArgs, DemoStartArgs, PROJECT_DIR,
};
use anyhow::{Context, Result};
use arch_program::pubkey::Pubkey;
use colored::*;
use std::fs;
use std::path::PathBuf;

pub async fn setup_demo_environment(
    args: &DemoStartArgs,
    config: &Config,
) -> Result<(PathBuf, String, String, String)> {
    println!("{}", "Setting up demo environment...".bold().green());

    println!(
        "Network type: {}",
        config.get_string("selected_network").unwrap()
    );

    let rpc_url = get_rpc_url(args, config);
    println!("Using RPC URL: {}", rpc_url);

    // Get project directory
    let project_dir = config
        .get_string("project.directory")
        .context("Failed to get project directory from config")?;
    let demo_dir = PathBuf::from(&project_dir).join("demo");

    // Create demo directory if it doesn't exist
    if !demo_dir.exists() {
        println!(
            "  {} Demo directory not found. Creating it...",
            "ℹ".bold().blue()
        );

        fs::create_dir_all(&demo_dir)?;
        println!(
            "  {} Created demo directory at {:?}",
            "✓".bold().green(),
            demo_dir
        );

        // Extract demo files from binary
        extract_project_files(&PROJECT_DIR, &demo_dir)?;

        // Rename the .env.example file to .env if it exists
        let env_example_file = PathBuf::from(&demo_dir).join("app/frontend/.env.example");
        if env_example_file.exists() {
            fs::rename(
                &env_example_file,
                PathBuf::from(&demo_dir).join("app/frontend/.env"),
            )?;
        }
    }

    // Change to the demo directory
    std::env::set_current_dir(&demo_dir).context("Failed to change to demo directory")?;

    let env_file = PathBuf::from(&demo_dir).join("app/frontend/.env");
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

    deploy_program_from_path(
        &PathBuf::from(&demo_dir).join("app/program"),
        config,
        Some((program_keypair.clone(), program_pubkey_bytes)),
        Some(rpc_url.clone()),
    )
    .await?;

    make_program_executable(
        &program_keypair,
        &program_pubkey_bytes,
        Some(rpc_url.clone()),
    )
    .await?;

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

fn get_rpc_url(args: &DemoStartArgs, config: &Config) -> String {
    // Priority: 1. Command line arg, 2. Config file, 3. Default constant
    args.rpc_url
        .clone()
        .or_else(|| config.get_string("leader_rpc_endpoint").ok())
        .unwrap_or_else(|| common::constants::NODE1_ADDRESS.to_string())
}
