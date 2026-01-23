use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::BuildSystem;
use crate::error::PackError;

pub fn build_uberjar(project_dir: &Path, system: BuildSystem) -> Result<PathBuf, PackError> {
    match system {
        BuildSystem::DepsEdn => build_deps_edn(project_dir),
        BuildSystem::Leiningen => build_leiningen(project_dir),
    }
}

fn build_deps_edn(project_dir: &Path) -> Result<PathBuf, PackError> {
    let output = Command::new("clojure")
        .args(["-T:build", "uber"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| PackError::BuildFailed(format!("failed to run clojure: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PackError::BuildFailed(format!("clojure -T:build uber failed:\n{stderr}")));
    }

    find_uberjar(project_dir)
}

fn build_leiningen(project_dir: &Path) -> Result<PathBuf, PackError> {
    let output = Command::new("lein")
        .arg("uberjar")
        .current_dir(project_dir)
        .output()
        .map_err(|e| PackError::BuildFailed(format!("failed to run lein: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PackError::BuildFailed(format!("lein uberjar failed:\n{stderr}")));
    }

    find_uberjar(project_dir)
}

fn find_uberjar(project_dir: &Path) -> Result<PathBuf, PackError> {
    let target_dir = project_dir.join("target");

    // Look for *-standalone.jar first (lein convention), then any uberjar
    if let Ok(entries) = std::fs::read_dir(&target_dir) {
        let mut candidates: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().map_or(false, |ext| ext == "jar")
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map_or(false, |n| n.contains("standalone") || n.contains("uber"))
            })
            .collect();

        candidates.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
        candidates.reverse();

        if let Some(jar) = candidates.into_iter().next() {
            return Ok(jar);
        }
    }

    // deps.edn default output
    let default = target_dir.join("app.jar");
    if default.exists() {
        return Ok(default);
    }

    Err(PackError::UberjarNotFound(target_dir))
}
