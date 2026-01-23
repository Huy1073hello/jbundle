use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::PackError;
use crate::jvm::cache::jdk_bin;

pub fn detect_modules(jdk_path: &Path, jar_path: &Path) -> Result<String, PackError> {
    let jdeps = jdk_bin(jdk_path, "jdeps");

    tracing::info!("detecting required modules with jdeps");

    let jar_str = jar_path
        .to_str()
        .ok_or_else(|| PackError::JdepsFailed("JAR path contains invalid UTF-8".into()))?;

    let output = Command::new(&jdeps)
        .args([
            "--print-module-deps",
            "--ignore-missing-deps",
            "--multi-release",
            "base",
            jar_str,
        ])
        .output()
        .map_err(|e| PackError::JdepsFailed(format!("failed to run jdeps: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // jdeps can fail on some JARs, fall back to java.base
        tracing::warn!("jdeps failed, falling back to common modules: {stderr}");
        return Ok("java.base,java.logging,java.sql,java.naming,java.management,java.instrument,java.desktop,java.xml,java.net.http".to_string());
    }

    let modules = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if modules.is_empty() {
        return Ok("java.base".to_string());
    }

    Ok(modules)
}

pub fn create_runtime(
    jdk_path: &Path,
    modules: &str,
    output_dir: &Path,
) -> Result<PathBuf, PackError> {
    let jlink_bin = jdk_bin(jdk_path, "jlink");
    let runtime_path = output_dir.join("runtime");

    if runtime_path.exists() {
        std::fs::remove_dir_all(&runtime_path)?;
    }

    tracing::info!("creating minimal JVM runtime with jlink");

    let runtime_str = runtime_path
        .to_str()
        .ok_or_else(|| PackError::JlinkFailed("runtime path contains invalid UTF-8".into()))?;

    let output = Command::new(&jlink_bin)
        .args([
            "--add-modules",
            modules,
            "--strip-debug",
            "--no-man-pages",
            "--no-header-files",
            "--compress=zip-6",
            "--output",
            runtime_str,
        ])
        .output()
        .map_err(|e| PackError::JlinkFailed(format!("failed to run jlink: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PackError::JlinkFailed(stderr.to_string()));
    }

    Ok(runtime_path)
}
