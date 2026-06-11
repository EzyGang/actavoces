use std::fs;

use serde::{Deserialize, Serialize};

use crate::worker::paths::{uv_runtime_is_available, worker_runtime_version, WorkerRuntimePaths};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkerBootstrapManifest {
    pub(crate) worker_version: String,
    pub(crate) worker_source_hash: String,
    pub(crate) uv_ready: bool,
    pub(crate) synced: bool,
    pub(crate) health_ok: bool,
    pub(crate) default_model: String,
    pub(crate) default_model_installed: bool,
}

pub(crate) fn worker_bootstrap_is_ready(paths: &WorkerRuntimePaths, source_hash: &str) -> bool {
    if !uv_runtime_is_available(paths)
        || !paths.worker_directory.join("app").join("main.py").exists()
    {
        return false;
    }

    match read_worker_bootstrap_manifest(paths) {
        Some(manifest) => {
            manifest.worker_version == worker_runtime_version()
                && manifest.worker_source_hash == source_hash
                && manifest.uv_ready
                && manifest.synced
                && manifest.health_ok
                && manifest.default_model == "medium"
                && manifest.default_model_installed
        }
        None => false,
    }
}

pub(crate) fn read_worker_bootstrap_manifest(
    paths: &WorkerRuntimePaths,
) -> Option<WorkerBootstrapManifest> {
    let manifest_path = paths.worker_directory.join("actavoces-worker-runtime.json");
    let content = fs::read_to_string(manifest_path).ok()?;

    serde_json::from_str(&content).ok()
}

pub(crate) fn write_worker_bootstrap_manifest(
    paths: &WorkerRuntimePaths,
    source_hash: &str,
) -> Result<(), String> {
    let manifest = WorkerBootstrapManifest {
        worker_version: worker_runtime_version(),
        worker_source_hash: source_hash.to_owned(),
        uv_ready: true,
        synced: true,
        health_ok: true,
        default_model: "medium".to_owned(),
        default_model_installed: true,
    };
    let content = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("Unable to serialize worker manifest: {error}"))?;

    fs::write(
        paths.worker_directory.join("actavoces-worker-runtime.json"),
        content,
    )
    .map_err(|error| format!("Unable to write worker manifest: {error}"))
}
