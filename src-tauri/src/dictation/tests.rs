use std::fs;
use std::time::SystemTime;

use crate::domain::types::DictationShortcutMode;
use crate::storage::repository::AppRepository;

use super::{
    cleanup_stale_dictations, transcript_body_for_test, DictationAction, DictationRuntime,
    DictationShortcutEvent, DictationState, MAX_DICTATION_DURATION,
};

#[test]
fn toggle_press_release_press_starts_then_stops() {
    let mut runtime = DictationRuntime::default();

    assert_eq!(
        runtime.shortcut_action(
            DictationShortcutMode::Toggle,
            DictationShortcutEvent::Pressed
        ),
        DictationAction::Start
    );
    runtime.set_state_for_test(DictationState::Capturing);
    assert_eq!(
        runtime.shortcut_action(
            DictationShortcutMode::Toggle,
            DictationShortcutEvent::Released
        ),
        DictationAction::Ignore
    );
    assert_eq!(
        runtime.shortcut_action(
            DictationShortcutMode::Toggle,
            DictationShortcutEvent::Pressed
        ),
        DictationAction::Stop
    );
}

#[test]
fn push_to_talk_starts_on_press_and_stops_on_release() {
    let mut runtime = DictationRuntime::default();

    assert_eq!(
        runtime.shortcut_action(
            DictationShortcutMode::PushToTalk,
            DictationShortcutEvent::Pressed
        ),
        DictationAction::Start
    );
    runtime.set_state_for_test(DictationState::Capturing);
    assert_eq!(
        runtime.shortcut_action(
            DictationShortcutMode::PushToTalk,
            DictationShortcutEvent::Released
        ),
        DictationAction::Stop
    );
}

#[test]
fn repeated_and_duplicate_shortcut_events_are_ignored() {
    let mut runtime = DictationRuntime::default();

    assert_eq!(
        runtime.shortcut_action(
            DictationShortcutMode::PushToTalk,
            DictationShortcutEvent::Pressed
        ),
        DictationAction::Start
    );
    runtime.set_state_for_test(DictationState::Capturing);
    assert_eq!(
        runtime.shortcut_action(
            DictationShortcutMode::PushToTalk,
            DictationShortcutEvent::Pressed
        ),
        DictationAction::Ignore
    );
    assert_eq!(
        runtime.shortcut_action(
            DictationShortcutMode::PushToTalk,
            DictationShortcutEvent::Released
        ),
        DictationAction::Stop
    );
    runtime.set_state_for_test(DictationState::Transcribing);
    assert_eq!(
        runtime.shortcut_action(
            DictationShortcutMode::PushToTalk,
            DictationShortcutEvent::Released
        ),
        DictationAction::Ignore
    );
}

#[test]
fn stale_release_does_not_stop_an_active_session() {
    let mut runtime = DictationRuntime::default();
    runtime.set_state_for_test(DictationState::Capturing);

    assert_eq!(
        runtime.shortcut_action(
            DictationShortcutMode::PushToTalk,
            DictationShortcutEvent::Released
        ),
        DictationAction::Ignore
    );
}

#[test]
fn finalization_and_transcription_block_new_sessions_and_recordings() {
    for state in [DictationState::Finalizing, DictationState::Transcribing] {
        let mut runtime = DictationRuntime::default();
        runtime.set_state_for_test(state);

        assert!(runtime.blocks_recording());
        assert_eq!(
            runtime.shortcut_action(
                DictationShortcutMode::Toggle,
                DictationShortcutEvent::Pressed
            ),
            DictationAction::Ignore
        );
    }
}

#[test]
fn maximum_duration_is_bounded() {
    let mut runtime = DictationRuntime::default();
    runtime.set_state_for_test(DictationState::Capturing);
    runtime.set_started_at_for_test(SystemTime::now() - MAX_DICTATION_DURATION);

    assert!(runtime.duration_limit_reached(SystemTime::now()));
}

#[test]
fn startup_cleanup_removes_stale_temporary_audio() {
    let root = std::env::temp_dir().join(format!(
        "actavoces-dictation-cleanup-{}",
        std::process::id()
    ));
    let stale_directory = root.join("dictation-runtime").join("stale-session");
    fs::create_dir_all(&stale_directory).unwrap();
    fs::write(stale_directory.join("dictation.wav"), b"temporary audio").unwrap();

    cleanup_stale_dictations(&root).unwrap();

    assert!(!root.join("dictation-runtime").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn copied_dictation_contains_plain_text_without_timestamps() {
    let transcript =
        "# Raw transcript - Dictation\n\n[00:00:00 - 00:00:01] Hello\n[00:00:01 - 00:00:02] world\n";

    assert_eq!(transcript_body_for_test(transcript), "Hello world");
}

#[test]
fn dictation_runtime_does_not_create_meeting_database_rows() {
    let root = std::env::temp_dir().join(format!(
        "actavoces-dictation-database-{}",
        std::process::id()
    ));
    let repository = AppRepository::open(&root.join("actavoces.sqlite")).unwrap();

    for table in [
        "recordings",
        "pipeline_jobs",
        "recording_artifacts",
        "job_events",
    ] {
        let count = repository
            .connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get::<_, u32>(0)
            })
            .unwrap();
        assert_eq!(count, 0, "{table} must remain empty");
    }

    drop(repository);
    let _ = fs::remove_dir_all(root);
}
