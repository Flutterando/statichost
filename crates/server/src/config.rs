use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub token: String,
    pub domain: String,
    pub port: u16,
    pub sites_dir: PathBuf,
    pub binaries_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let token = std::env::var("STATICHOST_TOKEN")
            .map_err(|_| anyhow::anyhow!("STATICHOST_TOKEN must be set"))?;
        if token.trim().is_empty() {
            anyhow::bail!("STATICHOST_TOKEN must not be empty");
        }
        let domain = std::env::var("STATICHOST_DOMAIN")
            .map_err(|_| anyhow::anyhow!("STATICHOST_DOMAIN must be set"))?;
        let port = std::env::var("STATICHOST_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000);
        let sites_dir = std::env::var("STATICHOST_SITES_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/sites"));
        let binaries_dir = std::env::var("STATICHOST_BINARIES_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/dl"));

        std::fs::create_dir_all(&sites_dir)
            .map_err(|e| anyhow::anyhow!("create sites_dir {sites_dir:?}: {e}"))?;
        std::fs::create_dir_all(&binaries_dir)
            .map_err(|e| anyhow::anyhow!("create binaries_dir {binaries_dir:?}: {e}"))?;

        Ok(Self {
            token,
            domain,
            port,
            sites_dir,
            binaries_dir,
        })
    }
}
