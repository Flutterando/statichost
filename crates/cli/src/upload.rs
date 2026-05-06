use std::path::Path;

use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;

pub fn make_tar_gz(src: &Path) -> Result<Vec<u8>> {
    let src = src
        .canonicalize()
        .with_context(|| format!("canonicalize {}", src.display()))?;
    if !src.is_dir() {
        anyhow::bail!("{} is not a directory", src.display());
    }
    let buf = Vec::new();
    let enc = GzEncoder::new(buf, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.follow_symlinks(false);
    tar.append_dir_all(".", &src)
        .with_context(|| format!("tar {}", src.display()))?;
    let enc = tar.into_inner()?;
    Ok(enc.finish()?)
}
