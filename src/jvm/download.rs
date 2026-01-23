use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::PackError;

use super::adoptium::ReleaseAsset;

pub async fn download_jdk(release: &ReleaseAsset) -> Result<PathBuf, PackError> {
    let url = &release.binary.package.link;
    let expected_sha = &release.binary.package.checksum;
    let file_name = &release.binary.package.name;

    let cache_dir = crate::config::BuildConfig::cache_dir();
    std::fs::create_dir_all(&cache_dir)?;
    let dest = cache_dir.join(file_name);

    if dest.exists() {
        if verify_checksum(&dest, expected_sha)? {
            tracing::info!("archive already downloaded and verified");
            return Ok(dest);
        }
        std::fs::remove_file(&dest)?;
    }

    tracing::info!("downloading JDK from {url}");

    let response = reqwest::get(url)
        .await
        .map_err(|e| PackError::JdkDownload(format!("download failed: {e}")))?;

    let total_size = response.content_length().unwrap_or(release.binary.package.size);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("=> "),
    );
    pb.set_message("Downloading JDK");

    let mut file = tokio::fs::File::create(&dest).await?;
    let stream = response.bytes().await
        .map_err(|e| PackError::JdkDownload(format!("download stream failed: {e}")))?;

    pb.inc(stream.len() as u64);
    file.write_all(&stream).await?;
    file.flush().await?;
    drop(file);

    pb.finish_with_message("Download complete");

    if !verify_checksum(&dest, expected_sha)? {
        std::fs::remove_file(&dest)?;
        return Err(PackError::ChecksumMismatch {
            expected: expected_sha.clone(),
            actual: "mismatch".into(),
        });
    }

    Ok(dest)
}

fn verify_checksum(path: &PathBuf, expected: &str) -> Result<bool, PackError> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = format!("{:x}", hasher.finalize());
    Ok(result == expected)
}
