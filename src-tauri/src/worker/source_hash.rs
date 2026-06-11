use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use crate::worker::paths::resolve_worker_resource_directory;

pub(crate) fn worker_source_hash(app: &tauri::AppHandle) -> Result<String, String> {
    let source = resolve_worker_resource_directory(app)?;

    hash_worker_source_directory(&source)
}

pub(crate) fn hash_worker_source_directory(source: &Path) -> Result<String, String> {
    let mut hasher = DefaultHasher::new();

    hash_directory(&source.join("app"), source, &mut hasher)?;
    hash_file(&source.join("pyproject.toml"), source, &mut hasher)?;
    hash_file(&source.join("uv.lock"), source, &mut hasher)?;

    Ok(format!("{:016x}", hasher.finish()))
}

pub(crate) fn hash_directory(
    source: &Path,
    root: &Path,
    hasher: &mut DefaultHasher,
) -> Result<(), String> {
    let mut entries = fs::read_dir(source)
        .map_err(|error| format!("Unable to read directory {}: {error}", source.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Unable to read directory entry: {error}"))?;

    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();

        if path.is_dir() {
            hash_directory(&path, root, hasher)?;
        } else {
            hash_file(&path, root, hasher)?;
        }
    }

    Ok(())
}

pub(crate) fn hash_file(
    path: &Path,
    root: &Path,
    hasher: &mut DefaultHasher,
) -> Result<(), String> {
    let relative_path = path.strip_prefix(root).unwrap_or(path);
    let content =
        fs::read(path).map_err(|error| format!("Unable to hash {}: {error}", path.display()))?;

    relative_path.hash(hasher);
    content.hash(hasher);

    Ok(())
}
