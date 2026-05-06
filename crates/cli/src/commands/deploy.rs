use std::path::PathBuf;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::multipart;
use serde::Deserialize;

use crate::api_client::Api;
use crate::config::load;
use crate::upload::make_tar_gz;

#[derive(Deserialize)]
struct DeployResp {
    name: String,
    url: String,
}

pub async fn run(path: PathBuf, name: String) -> Result<()> {
    let cfg = load()?;
    let api = Api::from_cfg(&cfg)?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb.set_message(format!("packing {}", path.display()));

    let archive = make_tar_gz(&path).context("create archive")?;
    let archive_len = archive.len();
    pb.set_message(format!("uploading {} ({} bytes)", name, archive_len));

    let part = multipart::Part::bytes(archive)
        .file_name("site.tar.gz")
        .mime_str("application/gzip")?;
    let form = multipart::Form::new()
        .text("name", name.clone())
        .part("archive", part);

    let resp = api
        .client
        .post(api.url("/api/deploy"))
        .multipart(form)
        .send()
        .await
        .context("send deploy")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        pb.finish_and_clear();
        anyhow::bail!("deploy failed: {status}: {body}");
    }
    pb.finish_and_clear();
    let parsed: DeployResp = serde_json::from_str(&body).context("parse deploy response")?;
    println!("Deployed: {} -> {}", parsed.name, parsed.url);
    Ok(())
}
