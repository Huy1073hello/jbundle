pub mod archive;
pub mod stub;

use std::path::Path;

use crate::error::PackError;

pub fn create_binary(
    runtime_dir: &Path,
    jar_path: &Path,
    output: &Path,
    jvm_args: &[String],
) -> Result<(), PackError> {
    let temp = tempfile::tempdir()?;

    let payload_path = archive::create_payload(runtime_dir, jar_path, temp.path())?;
    let payload_size = std::fs::metadata(&payload_path)?.len();
    let payload_hash = archive::hash_file(&payload_path)?;

    let stub_script = stub::generate(&payload_hash, payload_size, jvm_args);

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut out_file = std::fs::File::create(output)?;
    use std::io::Write;
    out_file.write_all(stub_script.as_bytes())?;

    let payload_data = std::fs::read(&payload_path)?;
    out_file.write_all(&payload_data)?;
    drop(out_file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(output, std::fs::Permissions::from_mode(0o755))?;
    }

    tracing::info!("binary created at {}", output.display());
    Ok(())
}
