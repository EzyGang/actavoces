use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

static DIAGNOSTICS_LOG: OnceLock<DiagnosticsLog> = OnceLock::new();

struct DiagnosticsLog {
    file: Mutex<File>,
}

pub(crate) fn initialize(log_directory: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(log_directory)
        .map_err(|error| format!("Unable to create diagnostics log directory: {error}"))?;

    let path = log_directory.join("actavoces-diagnostics.jsonl");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| format!("Unable to open diagnostics log: {error}"))?;

    let _ = DIAGNOSTICS_LOG.set(DiagnosticsLog {
        file: Mutex::new(file),
    });
    record("info", "app.diagnostics.ready", &path.display().to_string());

    Ok(path)
}

pub(crate) fn info(event: &str, message: &str) {
    record("info", event, message);
}

pub(crate) fn error(event: &str, message: &str) {
    record("error", event, message);
}

fn record(level: &str, event: &str, message: &str) {
    let Some(log) = DIAGNOSTICS_LOG.get() else {
        return;
    };
    let Ok(mut file) = log.file.lock() else {
        return;
    };

    let payload = json!({
        "timestamp": timestamp_millis(),
        "level": level,
        "event": event,
        "message": message,
    });

    if let Ok(line) = serde_json::to_string(&payload) {
        let _ = writeln!(file, "{line}");
    }
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}
