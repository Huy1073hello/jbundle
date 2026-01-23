pub mod adoptium;
pub mod cache;
pub mod download;

use std::path::PathBuf;

use crate::config::Target;
use crate::error::PackError;

pub async fn ensure_jdk(version: u8, target: &Target) -> Result<PathBuf, PackError> {
    let cache_path = cache::cached_jdk_path(version, target);
    if cache_path.exists() {
        tracing::info!("using cached JDK {} at {}", version, cache_path.display());
        return Ok(cache_path);
    }

    let release = adoptium::fetch_latest_release(version, target).await?;
    let archive_path = download::download_jdk(&release).await?;
    let jdk_path = cache::extract_and_cache(version, target, &archive_path)?;

    Ok(jdk_path)
}
