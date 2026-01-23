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
