use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::worker::command::{WorkerClient, WorkerProcess};
use crate::worker::paths::WorkerRuntimePaths;

#[test]
fn worker_process_reuses_one_child_for_multiple_commands() {
    let mut process = scripted_worker(
        "import json,sys\nfor line in sys.stdin:\n c=json.loads(line); print(json.dumps({'commandId':c['id'],'event':'health.ok','payload':{'pid':__import__('os').getpid()}}), flush=True)",
    );

    let first = run_health(&mut process, "one", Duration::from_secs(1)).unwrap();
    let second = run_health(&mut process, "two", Duration::from_secs(1)).unwrap();

    assert_eq!(first[0].payload["pid"], second[0].payload["pid"]);
    process.shutdown().unwrap();
}

#[test]
fn worker_client_restarts_after_failure_and_path_change() {
    let mut client = WorkerClient::new();
    let first_paths = test_paths();
    let mut second_paths = test_paths();
    second_paths.worker_directory = PathBuf::from("other");

    let first = client
        .run_with_process(&first_paths, "health.check", serde_json::json!({}), |paths| {
            Ok(scripted_worker_with_paths(
                "import json\nc=json.loads(input()); print(json.dumps({'commandId':c['id'],'event':'health.ok','payload':{'worker':'first'}}), flush=True)",
                paths.clone(),
            ))
        })
        .unwrap();
    let second = client
        .run_with_process(&second_paths, "health.check", serde_json::json!({}), |paths| {
            Ok(scripted_worker_with_paths(
                "import json\nc=json.loads(input()); print(json.dumps({'commandId':c['id'],'event':'health.ok','payload':{'worker':'second'}}), flush=True)",
                paths.clone(),
            ))
        })
        .unwrap();

    assert_eq!(first[0].payload["worker"], "first");
    assert_eq!(second[0].payload["worker"], "second");
    client.shutdown().unwrap();
}

#[test]
fn worker_client_restarts_after_command_failure() {
    let mut client = WorkerClient::new();
    let paths = test_paths();

    let error = client
        .run_with_process(&paths, "health.check", serde_json::json!({}), |paths| {
            Ok(scripted_worker_with_paths(
                "print('not-json', flush=True); input()",
                paths.clone(),
            ))
        })
        .unwrap_err();
    assert!(error.contains("Unable to parse worker event"));

    let events = client
        .run_with_process(&paths, "health.check", serde_json::json!({}), |paths| {
            Ok(scripted_worker_with_paths(
                "import json\nc=json.loads(input()); print(json.dumps({'commandId':c['id'],'event':'health.ok','payload':{}}), flush=True)",
                paths.clone(),
            ))
        })
        .unwrap();

    assert_eq!(events[0].event, "health.ok");
    client.shutdown().unwrap();
}
#[test]
fn worker_process_reports_malformed_output_and_can_restart() {
    let mut malformed = scripted_worker("print('not-json', flush=True); input()");

    let error = run_health(&mut malformed, "bad", Duration::from_secs(1)).unwrap_err();
    assert!(error.contains("Unable to parse worker event"));
    malformed.shutdown().unwrap();

    let mut restarted = scripted_worker(
        "import json,sys\nc=json.loads(input()); print(json.dumps({'commandId':c['id'],'event':'health.ok','payload':{}}), flush=True)",
    );
    assert!(run_health(&mut restarted, "good", Duration::from_secs(1)).is_ok());
    restarted.shutdown().unwrap();
}

#[test]
fn worker_process_reports_exit_and_timeout() {
    let mut exited = scripted_worker("raise SystemExit('worker crashed')");
    let exit_error = run_health(&mut exited, "exit", Duration::from_secs(1)).unwrap_err();
    assert!(exit_error.contains("Worker exited before completing command"));
    exited.shutdown().unwrap();

    let mut timed_out = scripted_worker("import time\ninput(); time.sleep(5)");
    let timeout_error =
        run_health(&mut timed_out, "timeout", Duration::from_millis(20)).unwrap_err();
    assert!(timeout_error.contains("timed out"));
    timed_out.shutdown().unwrap();
}

#[test]
fn worker_process_serializes_progress_and_clean_shutdown() {
    let mut process = scripted_worker(
        "import json\nc=json.loads(input())\nfor event in ('transcribe.progress','transcribe.complete'):\n print(json.dumps({'commandId':c['id'],'event':event,'payload':{}}), flush=True)",
    );
    let command = serde_json::json!({"id":"serial","name":"transcribe.run","payload":{}});

    let events = process
        .run("serial", "transcribe.run", &command, Duration::from_secs(1))
        .unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event, "transcribe.progress");
    assert_eq!(events[1].event, "transcribe.complete");
    process.shutdown().unwrap();
}

fn scripted_worker(script: &str) -> WorkerProcess {
    scripted_worker_with_paths(script, test_paths())
}

fn scripted_worker_with_paths(script: &str, paths: WorkerRuntimePaths) -> WorkerProcess {
    let mut command = Command::new("python3");
    command
        .arg("-u")
        .arg("-c")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    WorkerProcess::spawn(command, paths).unwrap()
}

fn run_health(
    process: &mut WorkerProcess,
    id: &str,
    timeout: Duration,
) -> Result<Vec<crate::domain::types::WorkerEvent>, String> {
    let command = serde_json::json!({"id":id,"name":"health.check","payload":{}});
    process.run(id, "health.check", &command, timeout)
}

fn test_paths() -> WorkerRuntimePaths {
    WorkerRuntimePaths {
        uv_executable: PathBuf::new(),
        worker_directory: PathBuf::new(),
        uv_state_directory: PathBuf::new(),
        ffmpeg_directory: None,
    }
}
