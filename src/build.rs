use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::BuildSystem;
use crate::error::PackError;

fn ensure_command_exists(cmd: &str) -> Result<(), PackError> {
    which::which(cmd).map_err(|_| {
        PackError::BuildFailed(format!(
            "command '{cmd}' not found in PATH. Please install it before running clj-pack."
        ))
    })?;
    Ok(())
}

pub fn build_uberjar(project_dir: &Path, system: BuildSystem) -> Result<PathBuf, PackError> {
    match system {
        BuildSystem::DepsEdn => build_deps_edn(project_dir),
        BuildSystem::Leiningen => build_leiningen(project_dir),
    }
}

fn build_deps_edn(project_dir: &Path) -> Result<PathBuf, PackError> {
    let strategy = detect_deps_strategy(project_dir);

    match strategy {
        DepsStrategy::ToolsBuild => run_tools_build(project_dir),
        DepsStrategy::Uberjar => run_depstar_uberjar(project_dir),
    }
}

#[derive(Debug)]
enum DepsStrategy {
    ToolsBuild,
    Uberjar,
}

fn detect_deps_strategy(project_dir: &Path) -> DepsStrategy {
    if project_dir.join("build.clj").exists() {
        return DepsStrategy::ToolsBuild;
    }

    let deps_path = project_dir.join("deps.edn");
    if let Ok(content) = std::fs::read_to_string(&deps_path) {
        if content.contains(":uberjar") {
            return DepsStrategy::Uberjar;
        }
    }

    DepsStrategy::ToolsBuild
}

fn run_tools_build(project_dir: &Path) -> Result<PathBuf, PackError> {
    ensure_command_exists("clojure")?;
    tracing::info!("running: clojure -T:build uber");

    let output = Command::new("clojure")
        .args(["-T:build", "uber"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| PackError::BuildFailed(format!("failed to run clojure: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PackError::BuildFailed(format!(
            "clojure -T:build uber failed:\n{stderr}"
        )));
    }

    find_uberjar(project_dir)
}

fn run_depstar_uberjar(project_dir: &Path) -> Result<PathBuf, PackError> {
    ensure_command_exists("clojure")?;
    tracing::info!("running: clojure -X:uberjar");

    let output = Command::new("clojure")
        .args(["-X:uberjar"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| PackError::BuildFailed(format!("failed to run clojure: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PackError::BuildFailed(format!(
            "clojure -X:uberjar failed:\n{stderr}"
        )));
    }

    find_uberjar(project_dir)
}

fn build_leiningen(project_dir: &Path) -> Result<PathBuf, PackError> {
    ensure_command_exists("lein")?;
    tracing::info!("running: lein uberjar");

    let output = Command::new("lein")
        .arg("uberjar")
        .current_dir(project_dir)
        .output()
        .map_err(|e| PackError::BuildFailed(format!("failed to run lein: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PackError::BuildFailed(format!(
            "lein uberjar failed:\n{stderr}"
        )));
    }

    find_uberjar(project_dir)
}

fn find_uberjar(project_dir: &Path) -> Result<PathBuf, PackError> {
    let target_dir = project_dir.join("target");

    if let Ok(entries) = std::fs::read_dir(&target_dir) {
        let mut candidates: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|ext| ext == "jar")
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.contains("standalone") || n.contains("uber"))
            })
            .collect();

        candidates.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
        candidates.reverse();

        if let Some(jar) = candidates.into_iter().next() {
            return Ok(jar);
        }
    }

    // Look for any .jar in target/ (depstar, custom names)
    if let Ok(entries) = std::fs::read_dir(&target_dir) {
        let mut jars: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|ext| ext == "jar")
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| !n.contains("sources") && !n.contains("javadoc"))
            })
            .collect();

        jars.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
        jars.reverse();

        if let Some(jar) = jars.into_iter().next() {
            return Ok(jar);
        }
    }

    Err(PackError::UberjarNotFound(target_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn find_uberjar_prefers_standalone_jar() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app.jar"), b"regular").unwrap();
        std::fs::write(target.join("app-standalone.jar"), b"standalone").unwrap();

        let result = find_uberjar(dir.path()).unwrap();
        assert!(result
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("standalone"));
    }

    #[test]
    fn find_uberjar_prefers_uber_jar() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app.jar"), b"regular").unwrap();
        std::fs::write(target.join("app-uber.jar"), b"uber").unwrap();

        let result = find_uberjar(dir.path()).unwrap();
        assert!(result
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("uber"));
    }

    #[test]
    fn find_uberjar_falls_back_to_any_jar() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("myapp.jar"), b"content").unwrap();

        let result = find_uberjar(dir.path()).unwrap();
        assert_eq!(result.file_name().unwrap(), "myapp.jar");
    }

    #[test]
    fn find_uberjar_excludes_sources_and_javadoc() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app-sources.jar"), b"src").unwrap();
        std::fs::write(target.join("app-javadoc.jar"), b"doc").unwrap();
        std::fs::write(target.join("app.jar"), b"app").unwrap();

        let result = find_uberjar(dir.path()).unwrap();
        assert_eq!(result.file_name().unwrap(), "app.jar");
    }

    #[test]
    fn find_uberjar_error_when_no_jars() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("readme.txt"), b"text").unwrap();

        let result = find_uberjar(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn find_uberjar_error_when_no_target_dir() {
        let dir = tempdir().unwrap();
        let result = find_uberjar(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn detect_deps_strategy_tools_build() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        std::fs::write(dir.path().join("build.clj"), "(ns build)").unwrap();

        let strategy = detect_deps_strategy(dir.path());
        assert!(matches!(strategy, DepsStrategy::ToolsBuild));
    }

    #[test]
    fn detect_deps_strategy_uberjar() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{:uberjar {:some :config}}").unwrap();

        let strategy = detect_deps_strategy(dir.path());
        assert!(matches!(strategy, DepsStrategy::Uberjar));
    }

    #[test]
    fn ensure_command_exists_finds_sh() {
        assert!(ensure_command_exists("sh").is_ok());
    }

    #[test]
    fn ensure_command_exists_fails_for_missing() {
        assert!(ensure_command_exists("nonexistent_binary_xyz_123").is_err());
    }
}
