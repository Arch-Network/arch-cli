use anyhow::{Context, Result};
use std::process::Command as ShellCommand;

pub async fn get_cloud_run_url(
    project_id: &str,
    region: &str,
    service_name: &str,
) -> Result<String> {
    let output = ShellCommand::new("gcloud")
        .args([
            "run",
            "services",
            "describe",
            service_name,
            "--platform",
            "managed",
            "--region",
            region,
            "--project",
            project_id,
            "--format",
            "get(status.url)",
        ])
        .output()
        .context("Failed to get Cloud Run service URL")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to get Cloud Run URL: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
