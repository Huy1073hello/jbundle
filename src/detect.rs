use std::path::Path;

use crate::config::BuildSystem;
use crate::error::PackError;

pub fn detect_build_system(project_dir: &Path) -> Result<BuildSystem, PackError> {
    if project_dir.join("deps.edn").exists() {
        return Ok(BuildSystem::DepsEdn);
    }
    if project_dir.join("project.clj").exists() {
        return Ok(BuildSystem::Leiningen);
    }
    Err(PackError::NoBuildSystem(project_dir.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn detects_deps_edn() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        let result = detect_build_system(dir.path()).unwrap();
        assert_eq!(result, BuildSystem::DepsEdn);
    }

    #[test]
    fn detects_leiningen() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("project.clj"), "(defproject foo)").unwrap();
        let result = detect_build_system(dir.path()).unwrap();
        assert_eq!(result, BuildSystem::Leiningen);
    }

    #[test]
    fn deps_edn_has_priority_over_project_clj() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        std::fs::write(dir.path().join("project.clj"), "(defproject foo)").unwrap();
        let result = detect_build_system(dir.path()).unwrap();
        assert_eq!(result, BuildSystem::DepsEdn);
    }

    #[test]
    fn error_when_no_build_system() {
        let dir = tempdir().unwrap();
        let result = detect_build_system(dir.path());
        assert!(result.is_err());
    }
}
