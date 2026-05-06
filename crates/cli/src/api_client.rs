use anyhow::{Context, Result};
use reqwest::{header, Client};

use crate::config::CliConfig;

pub struct Api {
    pub host: String,
    pub client: Client,
}

impl Api {
    pub fn from_cfg(cfg: &CliConfig) -> Result<Self> {
        let client = Client::builder()
            .default_headers({
                let mut h = header::HeaderMap::new();
                h.insert(
                    header::AUTHORIZATION,
                    header::HeaderValue::from_str(&format!("Bearer {}", cfg.token))?,
                );
                h
            })
            .build()
            .context("build http client")?;
        Ok(Self {
            host: cfg.host.trim_end_matches('/').to_string(),
            client,
        })
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.host, path)
    }
}
