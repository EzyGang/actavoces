use crate::domain::types::{ModelInventoryItem, RuntimeCapabilities, WorkerEvent};

pub(crate) fn parse_worker_events(output: &str) -> Result<Vec<WorkerEvent>, String> {
    let mut events = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        events.push(
            serde_json::from_str(line)
                .map_err(|error| format!("Unable to parse worker event: {error}"))?,
        );
    }

    Ok(events)
}

pub(crate) fn extract_model_inventory(
    events: &[WorkerEvent],
) -> Result<Vec<ModelInventoryItem>, String> {
    for event in events {
        if event.event != "models.status" {
            continue;
        }

        return serde_json::from_value(
            event
                .payload
                .get("models")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
        )
        .map_err(|error| format!("Unable to parse model status: {error}"));
    }

    Err("Worker did not return model status".to_owned())
}

pub(crate) fn extract_runtime_capabilities(
    events: &[WorkerEvent],
) -> Result<RuntimeCapabilities, String> {
    for event in events {
        if event.event != "runtime.capabilities" {
            continue;
        }

        return serde_json::from_value(event.payload.clone())
            .map_err(|error| format!("Unable to parse runtime capabilities: {error}"));
    }

    Err("Worker did not return runtime capabilities".to_owned())
}

pub(crate) fn model_install_message(events: &[WorkerEvent]) -> String {
    for event in events {
        if event.event == "models.install.needs_setup" {
            let dependency = event
                .payload
                .get("dependency")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("faster-whisper");

            return format!("Model installation requires {dependency}");
        }

        if event.event == "command.failed" {
            return event
                .payload
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Model installation failed")
                .to_owned();
        }
    }

    "Model installation did not complete".to_owned()
}

pub(crate) fn diarization_setup_message(events: &[WorkerEvent]) -> String {
    for event in events {
        if event.event == "diarization.needs_setup" {
            return event
                .payload
                .get("message")
                .and_then(serde_json::Value::as_str)
                .or_else(|| {
                    event
                        .payload
                        .get("dependency")
                        .and_then(serde_json::Value::as_str)
                })
                .unwrap_or("Speaker diarization setup is incomplete")
                .to_owned();
        }

        if event.event == "command.failed" {
            return event
                .payload
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Speaker diarization setup failed")
                .to_owned();
        }
    }

    "Speaker diarization setup did not complete".to_owned()
}
