use anyhow::{Context, Result};
use dialoguer::Confirm;

use crate::api_client::Api;
use crate::config::load;

pub async fn run(name: String, force: bool) -> Result<()> {
    if !force {
        let ok = Confirm::new()
            .with_prompt(format!("Delete site '{name}'?"))
            .default(false)
            .interact()?;
        if !ok {
            println!("Cancelled.");
            return Ok(());
        }
    }
    let cfg = load()?;
    let api = Api::from_cfg(&cfg)?;
    let resp = api
        .client
        .delete(api.url(&format!("/api/sites/{name}")))
        .send()
        .await
        .context("send delete")?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("delete failed: {status}: {body}");
    }
    println!("Deleted '{name}'");
    Ok(())
}
