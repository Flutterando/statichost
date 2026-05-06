use anyhow::{Context, Result};

use crate::config::{save, CliConfig};

pub async fn run(host: String, token: String) -> Result<()> {
    let host = host.trim_end_matches('/').to_string();

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{host}/api/sites"))
        .bearer_auth(&token)
        .send()
        .await
        .context("connect to server")?;

    if !resp.status().is_success() {
        anyhow::bail!("server rejected token: HTTP {}", resp.status());
    }

    save(&CliConfig {
        host: host.clone(),
        token,
    })?;
    println!("Logged in to {host}");
    Ok(())
}
