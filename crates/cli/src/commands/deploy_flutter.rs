use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;

pub async fn run(name: String) -> Result<()> {
    println!("Running: flutter build web");
    let status = Command::new("flutter")
        .args(["build", "web"])
        .status()?;
    if !status.success() {
        anyhow::bail!("flutter build web failed");
    }
    let path = PathBuf::from("./build/web");
    if !path.is_dir() {
        anyhow::bail!("./build/web not found after flutter build");
    }
    super::deploy::run(path, name).await
}
