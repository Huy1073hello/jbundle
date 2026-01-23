use std::path::{Path, PathBuf};

use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};

use crate::error::PackError;

pub fn create_payload(runtime_dir: &Path, jar_path: &Path, work_dir: &Path) -> Result<PathBuf, PackError> {
    let payload_path = work_dir.join("payload.tar.gz");
    let file = std::fs::File::create(&payload_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(encoder);

    tracing::info!("creating payload archive");

    // Add runtime directory
    tar.append_dir_all("runtime", runtime_dir)?;

    // Add JAR as app.jar
    tar.append_path_with_name(jar_path, "app.jar")?;

    let encoder = tar.into_inner()?;
    encoder.finish()?;

    Ok(payload_path)
}

pub fn hash_file(path: &Path) -> Result<String, PackError> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = format!("{:x}", hasher.finalize());
    Ok(hash[..16].to_string())
}
