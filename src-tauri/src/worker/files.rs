use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::worker::paths::{resolve_uv_resource_executable, resolve_worker_resource_directory};

pub(crate) fn prepare_worker_directory(
    app: &tauri::AppHandle,
    target: &Path,
) -> Result<(), String> {
    fs::create_dir_all(target)
        .map_err(|error| format!("Unable to create worker directory: {error}"))?;

    let source = resolve_worker_resource_directory(app)?;
    copy_directory(&source.join("app"), &target.join("app"))?;
    copy_file(
        &source.join("pyproject.toml"),
        &target.join("pyproject.toml"),
    )?;
    copy_file(&source.join("uv.lock"), &target.join("uv.lock"))
}

pub(crate) fn prepare_uv_executable(app: &tauri::AppHandle, target: &Path) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Unable to create uv runtime directory: {error}"))?;
    }

    match resolve_uv_resource_executable(app) {
        Ok(source) => copy_file(&source, target)?,
        Err(_) => {
            let status = Command::new("uv")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|error| {
                    format!("Bundled uv was not found and PATH uv is unavailable: {error}")
                })?;

            if !status.success() {
                return Err(
                    "Bundled uv was not found and PATH uv did not run successfully".to_owned(),
                );
            }

            return Ok(());
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(target)
            .map_err(|error| format!("Unable to read uv permissions: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(target, permissions)
            .map_err(|error| format!("Unable to set uv executable permissions: {error}"))?;
    }

    Ok(())
}

pub(crate) fn copy_directory(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        fs::remove_dir_all(target).map_err(|error| {
            format!("Unable to replace directory {}: {error}", target.display())
        })?;
    }

    fs::create_dir_all(target)
        .map_err(|error| format!("Unable to create directory {}: {error}", target.display()))?;

    for entry in fs::read_dir(source)
        .map_err(|error| format!("Unable to read directory {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("Unable to read directory entry: {error}"))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory(&source_path, &target_path)?;
        } else {
            copy_file(&source_path, &target_path)?;
        }
    }

    Ok(())
}

pub(crate) fn copy_file(source: &Path, target: &Path) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Unable to create directory {}: {error}", parent.display()))?;
    }

    fs::copy(source, target).map_err(|error| {
        format!(
            "Unable to copy {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;

    Ok(())
}
