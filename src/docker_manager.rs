use std::process::Command;
use anyhow::{ Result, Context };
use std::env;

pub fn start_docker_compose(file_path: &str) -> Result<()> {
    if !is_docker_compose_installed() {
        return Err(anyhow::anyhow!("Docker Compose is not installed or not in PATH"));
    }

    if !std::path::Path::new(file_path).exists() {
        return Err(anyhow::anyhow!("Docker Compose file not found: {}", file_path));
    }

    println!("Starting Docker Compose with file: {}", file_path);
    println!("Current directory: {}", env::current_dir()?.display());

    Command::new("docker-compose")
        .arg("-f")
        .arg(file_path)
        .arg("up")
        .arg("-d")
        .status()
        .context("Failed to execute docker-compose command")?;

    Ok(())
}

pub fn stop_docker_compose(file_path: &str) -> Result<()> {
    Command::new("docker-compose").arg("-f").arg(file_path).arg("down").status()?;
    Ok(())
}

fn is_docker_compose_installed() -> bool {
    Command::new("docker-compose").arg("--version").output().is_ok()
}
