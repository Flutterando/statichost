use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    pub host: String,
    pub token: String,
}

pub fn config_path() -> Result<PathBuf> {
    let dirs = directories::BaseDirs::new().context("no home directory")?;
    Ok(dirs.home_dir().join(".statichost").join("config.toml"))
}

pub fn load() -> Result<CliConfig> {
    let path = config_path()?;
    let txt = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}: run `statichost login` first", path.display()))?;
    let cfg: CliConfig = toml::from_str(&txt).context("parse config.toml")?;
    Ok(cfg)
}

pub fn save(cfg: &CliConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let txt = toml::to_string_pretty(cfg)?;
    std::fs::write(&path, txt)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(())
}
