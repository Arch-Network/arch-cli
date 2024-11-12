use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub fn build_frontend(
    demo_dir: &PathBuf,
    rpc_url: Option<&str>,
    program_pubkey: &str,
    wall_pubkey: &str,
) -> Result<()> {
    // Update .env file with production values
    let env_file = demo_dir.join("app/frontend/.env");
    let mut env_content = fs::read_to_string(&env_file)?;

    env_content = env_content
        .replace(
            "VITE_PROGRAM_PUBKEY=",
            &format!("VITE_PROGRAM_PUBKEY={}", program_pubkey),
        )
        .replace(
            "VITE_WALL_ACCOUNT_PUBKEY=",
            &format!("VITE_WALL_ACCOUNT_PUBKEY={}", wall_pubkey),
        );

    if let Some(url) = rpc_url {
        env_content = env_content.replace("VITE_RPC_URL=", &format!("VITE_RPC_URL={}", url));
    }

    fs::write(&env_file, env_content)?;

    Ok(())
}
