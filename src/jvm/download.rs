use std::path::PathBuf;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::PackError;

use super::adoptium::ReleaseAsset;

pub async fn download_jdk(
    release: &ReleaseAsset,
    mp: &MultiProgress,
) -> Result<PathBuf, PackError> {
    let url = &release.binary.package.link;
    let expected_sha = &release.binary.package.checksum;
    let file_name = &release.binary.package.name;

    let cache_dir = crate::config::BuildConfig::cache_dir()?;
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

    let total_size = response
        .content_length()
        .unwrap_or(release.binary.package.size);

    let pb = mp.add(ProgressBar::new(total_size));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  {msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("invalid progress bar template")
            .progress_chars("=> "),
    );
    pb.set_message("Downloading JDK");

    let mut file = tokio::fs::File::create(&dest).await?;
    let mut hasher = Sha256::new();

    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| PackError::JdkDownload(format!("download stream failed: {e}")))?
    {
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);
    }
    file.flush().await?;

    pb.finish_and_clear();

    let actual_hash = format!("{:x}", hasher.finalize());
    if actual_hash != *expected_sha {
        std::fs::remove_file(&dest)?;
        return Err(PackError::ChecksumMismatch {
            expected: expected_sha.clone(),
            actual: actual_hash,
        });
    }

    Ok(dest)
}

fn verify_checksum(path: &PathBuf, expected: &str) -> Result<bool, PackError> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    std::io::copy(&mut reader, &mut hasher)?;
    let result = format!("{:x}", hasher.finalize());
    Ok(result == expected)
}
