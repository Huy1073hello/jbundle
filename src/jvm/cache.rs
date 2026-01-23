use std::path::{Path, PathBuf};

use crate::config::{BuildConfig, Target};
use crate::error::PackError;

pub fn cached_jdk_path(version: u8, target: &Target) -> PathBuf {
    let dir_name = format!("jdk-{}-{}-{}", version, target.adoptium_os(), target.adoptium_arch());
    BuildConfig::cache_dir().join(dir_name)
}

pub fn extract_and_cache(version: u8, target: &Target, archive: &Path) -> Result<PathBuf, PackError> {
    let dest = cached_jdk_path(version, target);
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    std::fs::create_dir_all(&dest)?;

    let file_name = archive.file_name().unwrap().to_str().unwrap();

    if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        extract_tar_gz(archive, &dest)?;
    } else if file_name.ends_with(".zip") {
        extract_zip(archive, &dest)?;
    } else {
        return Err(PackError::JdkDownload(format!("unknown archive format: {file_name}")));
    }

    // Adoptium archives have a top-level directory, flatten it
    flatten_single_subdir(&dest)?;

    Ok(dest)
}

fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), PackError> {
    let file = std::fs::File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(dest)?;
    Ok(())
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<(), PackError> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;
    zip.extract(dest)?;
    Ok(())
}

fn flatten_single_subdir(dir: &Path) -> Result<(), PackError> {
    let entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();

    if entries.len() == 1 && entries[0].path().is_dir() {
        let subdir = entries[0].path();
        let temp = dir.join("__flatten_temp__");
        std::fs::rename(&subdir, &temp)?;

        for entry in std::fs::read_dir(&temp)? {
            let entry = entry?;
            std::fs::rename(entry.path(), dir.join(entry.file_name()))?;
        }
        std::fs::remove_dir(&temp)?;
    }

    Ok(())
}

pub fn jdk_bin(jdk_path: &Path, tool: &str) -> PathBuf {
    // macOS JDK has Contents/Home structure
    let macos_bin = jdk_path.join("Contents").join("Home").join("bin").join(tool);
    if macos_bin.exists() {
        return macos_bin;
    }
    jdk_path.join("bin").join(tool)
}
