use anyhow::Result;
use regex::Regex;
use std::fs;
use std::path::PathBuf;

pub fn build_frontend(
    demo_dir: &PathBuf,
    rpc_url: Option<&str>,
    program_pubkey: &str,
    wall_pubkey: &str,
    network: &str,
) -> Result<()> {
    // Update .env file with production values
    let env_file = demo_dir.join("app/frontend/.env");
    let mut env_content = fs::read_to_string(&env_file)?;

    env_content = Regex::new(r"VITE_PROGRAM_PUBKEY=.*")
        .unwrap()
        .replace(&env_content,&format!("VITE_PROGRAM_PUBKEY={}", program_pubkey))
        .to_string();
    env_content = Regex::new(r"VITE_WALL_ACCOUNT_PUBKEY=.*")
        .unwrap()
        .replace(&env_content,&format!("VITE_WALL_ACCOUNT_PUBKEY={}", wall_pubkey))
        .to_string();
    env_content = Regex::new(r"VITE_NETWORK=.*")
        .unwrap()
        .replace(&env_content, &format!("VITE_NETWORK={}", network))
        .to_string();

    if let Some(url) = rpc_url {
        let re = Regex::new(r"VITE_RPC_URL=.*").unwrap();
        env_content = re
            .replace(&env_content, &format!("VITE_RPC_URL={}", url))
            .to_string();
    }

    fs::write(&env_file, env_content)?;

    Ok(())
}
