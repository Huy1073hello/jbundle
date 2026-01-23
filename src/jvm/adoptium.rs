use serde::Deserialize;

use crate::config::Target;
use crate::error::PackError;

const ADOPTIUM_API: &str = "https://api.adoptium.net/v3";

#[derive(Debug, Deserialize)]
pub struct ReleaseAsset {
    pub binary: Binary,
}

#[derive(Debug, Deserialize)]
pub struct Binary {
    pub package: Package,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub link: String,
    pub checksum: String,
    pub size: u64,
    pub name: String,
}

pub async fn fetch_latest_release(version: u8, target: &Target) -> Result<ReleaseAsset, PackError> {
    let os = target.adoptium_os();
    let arch = target.adoptium_arch();

    let url = format!(
        "{ADOPTIUM_API}/assets/latest/{version}/hotspot\
         ?architecture={arch}&image_type=jdk&os={os}&vendor=eclipse"
    );

    tracing::debug!("fetching Adoptium release info: {url}");

    let assets: Vec<ReleaseAsset> = reqwest::get(&url)
        .await
        .map_err(|e| PackError::JdkDownload(format!("API request failed: {e}")))?
        .json()
        .await
        .map_err(|e| PackError::JdkDownload(format!("failed to parse API response: {e}")))?;

    assets
        .into_iter()
        .next()
        .ok_or_else(|| PackError::JdkDownload(format!("no JDK {version} found for {os}/{arch}")))
}
