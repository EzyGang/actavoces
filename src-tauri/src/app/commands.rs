pub(crate) mod models;
pub(crate) mod overlay;
pub(crate) mod pipeline;
pub(crate) mod recordings;
pub(crate) mod settings;
#[cfg(test)]
mod settings_tests;
pub(crate) mod snapshot;
pub(crate) mod speaker_labels;
pub(crate) mod tray;
pub(crate) mod worker;

pub use overlay::{create_recording_overlay, sync_recording_overlay};
pub use pipeline::{emit_snapshot_update, spawn_pipeline_processing};
pub use settings::{register_global_hotkeys, sync_launch_at_login};
pub use tray::{init_tray, sync_tray_recording_icon};

#[cfg(test)]
pub(crate) use pipeline::{normalized_transcription_context, resume_pipeline_jobs};
#[cfg(test)]
pub(crate) use recordings::{
    rename_recording_outputs, start_recording_session, stop_recording_session,
};
#[cfg(test)]
pub(crate) use speaker_labels::rewrite_speaker_label;
