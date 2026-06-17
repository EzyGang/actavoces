use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::worker::paths::{
    resolve_uv_resource_executable, resolve_worker_resource_directory, WorkerRuntimePaths,
};
use crate::worker::process::hide_console_window;
#[cfg(windows)]
use crate::worker::python::find_worker_python_executable;

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
            let mut command = Command::new("uv");
            hide_console_window(&mut command);
            if let Some(parent) = target.parent() {
                command.current_dir(parent);
            }
            let status = command
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

pub(crate) fn prepare_worker_virtualenv(paths: &WorkerRuntimePaths) -> Result<(), String> {
    if worker_virtualenv_is_scoped(paths) {
        return Ok(());
    }

    let venv_directory = paths.worker_directory.join(".venv");
    if !venv_directory.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(&venv_directory).map_err(|error| {
        format!(
            "Unable to inspect worker virtualenv {}: {error}",
            venv_directory.display()
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Refusing to replace symlinked worker virtualenv: {}",
            venv_directory.display()
        ));
    }

    let worker_directory = normalize_existing_path(&paths.worker_directory)?;
    let venv_parent = normalize_existing_path(
        venv_directory
            .parent()
            .ok_or_else(|| "Worker virtualenv parent is unavailable".to_owned())?,
    )?;

    if worker_directory != venv_parent {
        return Err(format!(
            "Refusing to replace virtualenv outside worker directory: {}",
            venv_directory.display()
        ));
    }

    fs::remove_dir_all(&venv_directory).map_err(|error| {
        format!(
            "Unable to replace worker virtualenv {}: {error}",
            venv_directory.display()
        )
    })
}

#[cfg(windows)]
pub(crate) fn repair_worker_virtualenv_python_home(
    paths: &WorkerRuntimePaths,
) -> Result<(), String> {
    let pyvenv_path = paths.worker_directory.join(".venv").join("pyvenv.cfg");
    let content = fs::read_to_string(&pyvenv_path).map_err(|error| {
        format!(
            "Unable to read worker virtualenv config {}: {error}",
            pyvenv_path.display()
        )
    })?;
    let Some(home) = pyvenv_home(&content) else {
        return Ok(());
    };
    let home_path = PathBuf::from(home);

    if !path_is_reparse_point(&home_path) {
        return Ok(());
    }

    let python_executable = find_worker_python_executable(paths)?
        .ok_or_else(|| "Worker Python installation was not found".to_owned())?;
    let python_home = python_executable
        .parent()
        .ok_or_else(|| "Worker Python executable parent is unavailable".to_owned())?;
    let updated = rewrite_pyvenv_home(&content, python_home);

    fs::write(&pyvenv_path, updated).map_err(|error| {
        format!(
            "Unable to write worker virtualenv config {}: {error}",
            pyvenv_path.display()
        )
    })
}

#[cfg(not(windows))]
pub(crate) fn repair_worker_virtualenv_python_home(
    _paths: &WorkerRuntimePaths,
) -> Result<(), String> {
    Ok(())
}

pub(crate) fn worker_virtualenv_is_scoped(paths: &WorkerRuntimePaths) -> bool {
    let pyvenv_path = paths.worker_directory.join(".venv").join("pyvenv.cfg");
    let content = match fs::read_to_string(pyvenv_path) {
        Ok(content) => content,
        Err(_) => return false,
    };

    let Some(home) = pyvenv_home(&content) else {
        return false;
    };

    let home_path = PathBuf::from(home);
    if !home_path.is_absolute() {
        return false;
    }

    if path_is_reparse_point(&home_path) {
        return false;
    }

    let expected_root = paths.uv_state_directory.join("python");

    match (
        normalize_existing_path(&home_path),
        normalize_existing_path(&expected_root),
    ) {
        (Ok(home), Ok(root)) => home.starts_with(root),
        _ => false,
    }
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

fn normalize_existing_path(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path)
        .map_err(|error| format!("Unable to resolve path {}: {error}", path.display()))
}

#[cfg(windows)]
fn path_is_reparse_point(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;

    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn path_is_reparse_point(_path: &Path) -> bool {
    false
}

fn pyvenv_home(content: &str) -> Option<&str> {
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim() == "home" {
            return Some(value.trim());
        }
    }

    None
}

pub(crate) fn rewrite_pyvenv_home(content: &str, home: &Path) -> String {
    let replacement = format!("home = {}", home.display());
    let mut updated = Vec::new();
    let mut replaced = false;

    for line in content.lines() {
        match line.split_once('=') {
            Some((key, _)) if key.trim() == "home" => {
                updated.push(replacement.clone());
                replaced = true;
            }
            _ => updated.push(line.to_owned()),
        }
    }

    if !replaced {
        updated.insert(0, replacement);
    }

    let mut next_content = updated.join("\n");
    if content.ends_with('\n') {
        next_content.push('\n');
    }

    next_content
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
