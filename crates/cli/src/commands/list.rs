use anyhow::{Context, Result};
use serde::Deserialize;

use crate::api_client::Api;
use crate::config::load;

#[derive(Deserialize)]
struct SiteInfo {
    name: String,
    files: u64,
    bytes: u64,
}

pub async fn run() -> Result<()> {
    let cfg = load()?;
    let api = Api::from_cfg(&cfg)?;
    let resp = api
        .client
        .get(api.url("/api/sites"))
        .send()
        .await
        .context("fetch sites")?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("list failed: {status}");
    }
    let sites: Vec<SiteInfo> = resp.json().await.context("parse sites")?;
    if sites.is_empty() {
        println!("No sites deployed.");
        return Ok(());
    }
    println!("{:<24} {:>8} {:>12}", "NAME", "FILES", "SIZE");
    for s in sites {
        println!("{:<24} {:>8} {:>12}", s.name, s.files, human(s.bytes));
    }
    Ok(())
}

fn human(b: u64) -> String {
    const K: u64 = 1024;
    if b < K {
        format!("{b} B")
    } else if b < K * K {
        format!("{:.1} KB", b as f64 / K as f64)
    } else if b < K * K * K {
        format!("{:.1} MB", b as f64 / (K * K) as f64)
    } else {
        format!("{:.2} GB", b as f64 / (K * K * K) as f64)
    }
}
