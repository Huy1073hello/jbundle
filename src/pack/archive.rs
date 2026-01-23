use std::path::{Path, PathBuf};

use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};

use crate::error::PackError;

pub fn create_payload(
    runtime_dir: &Path,
    jar_path: &Path,
    work_dir: &Path,
) -> Result<PathBuf, PackError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn hash_file_is_deterministic() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.bin");
        std::fs::write(&file, b"hello world").unwrap();

        let h1 = hash_file(&file).unwrap();
        let h2 = hash_file(&file).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_file_is_16_chars() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.bin");
        std::fs::write(&file, b"content").unwrap();

        let hash = hash_file(&file).unwrap();
        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn hash_file_different_content_different_hash() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("a.bin");
        let f2 = dir.path().join("b.bin");
        std::fs::write(&f1, b"content A").unwrap();
        std::fs::write(&f2, b"content B").unwrap();

        let h1 = hash_file(&f1).unwrap();
        let h2 = hash_file(&f2).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn create_payload_produces_tar_gz() {
        let dir = tempdir().unwrap();
        let runtime = dir.path().join("runtime");
        std::fs::create_dir_all(runtime.join("bin")).unwrap();
        std::fs::write(runtime.join("bin").join("java"), b"fake java").unwrap();

        let jar = dir.path().join("app.jar");
        std::fs::write(&jar, b"fake jar content").unwrap();

        let work = tempdir().unwrap();
        let payload = create_payload(&runtime, &jar, work.path()).unwrap();

        assert!(payload.exists());
        assert!(std::fs::metadata(&payload).unwrap().len() > 0);
    }

    #[test]
    fn create_payload_contains_runtime_and_jar() {
        let dir = tempdir().unwrap();
        let runtime = dir.path().join("runtime");
        std::fs::create_dir_all(runtime.join("bin")).unwrap();
        std::fs::write(runtime.join("bin").join("java"), b"fake").unwrap();

        let jar = dir.path().join("myapp.jar");
        std::fs::write(&jar, b"jar data").unwrap();

        let work = tempdir().unwrap();
        let payload = create_payload(&runtime, &jar, work.path()).unwrap();

        let file = std::fs::File::open(&payload).unwrap();
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        let entries: Vec<String> = archive
            .entries()
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(entries.iter().any(|e| e == "app.jar"));
        assert!(entries.iter().any(|e| e.starts_with("runtime/")));
    }
}
