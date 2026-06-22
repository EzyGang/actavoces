use serde::Serialize;
use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Size, WebviewUrl};

use crate::domain::types::{OverlayDisplayMode, OverlayPosition};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordingOverlaySyncPayload {
    visible: bool,
    display_mode: OverlayDisplayMode,
}

pub fn create_recording_overlay(app: &tauri::App) -> tauri::Result<()> {
    tauri::WebviewWindowBuilder::new(
        app,
        "recording-overlay",
        WebviewUrl::App("index.html".into()),
    )
    .title("ActaVoces recording")
    .inner_size(380.0, 72.0)
    .position(24.0, 24.0)
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()?;

    Ok(())
}

pub fn sync_recording_overlay(
    app: &tauri::AppHandle,
    visible: bool,
    position: OverlayPosition,
    display_mode: OverlayDisplayMode,
) -> Result<(), String> {
    let Some(overlay) = app.get_webview_window("recording-overlay") else {
        return Ok(());
    };

    if !visible || display_mode == OverlayDisplayMode::None {
        emit_recording_overlay_sync(&overlay, visible, display_mode)?;

        return overlay.hide().map_err(|error| error.to_string());
    }

    size_recording_overlay(&overlay, display_mode)?;
    position_recording_overlay(app, &overlay, position)?;
    emit_recording_overlay_sync(&overlay, visible, display_mode)?;
    overlay.show().map_err(|error| error.to_string())
}

fn emit_recording_overlay_sync(
    overlay: &tauri::WebviewWindow,
    visible: bool,
    display_mode: OverlayDisplayMode,
) -> Result<(), String> {
    overlay
        .emit(
            "recording-overlay-sync",
            RecordingOverlaySyncPayload {
                visible,
                display_mode,
            },
        )
        .map_err(|error| error.to_string())
}

fn size_recording_overlay(
    overlay: &tauri::WebviewWindow,
    display_mode: OverlayDisplayMode,
) -> Result<(), String> {
    let size = match display_mode {
        OverlayDisplayMode::Full => Size::Logical(LogicalSize::new(300.0, 72.0)),
        OverlayDisplayMode::Minimal => Size::Physical(PhysicalSize::new(64, 64)),
        OverlayDisplayMode::None => Size::Logical(LogicalSize::new(380.0, 72.0)),
    };

    overlay.set_size(size).map_err(|error| error.to_string())
}

fn position_recording_overlay(
    app: &tauri::AppHandle,
    overlay: &tauri::WebviewWindow,
    position: OverlayPosition,
) -> Result<(), String> {
    let margin = 24;
    let size = overlay.outer_size().map_err(|error| error.to_string())?;
    let monitor = app
        .get_webview_window("main")
        .and_then(|window| window.current_monitor().ok().flatten())
        .or_else(|| overlay.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        overlay
            .set_position(PhysicalPosition::new(margin, margin))
            .map_err(|error| error.to_string())?;

        return Ok(());
    };
    let monitor_origin = monitor.position();
    let monitor_size = monitor.size();
    let left = monitor_origin.x + margin;
    let right = monitor_origin.x + monitor_size.width as i32 - size.width as i32 - margin;
    let top = monitor_origin.y + margin;
    let bottom = monitor_origin.y + monitor_size.height as i32 - size.height as i32 - margin;
    let next_position = match position {
        OverlayPosition::TopLeft => PhysicalPosition::new(left, top),
        OverlayPosition::TopRight => PhysicalPosition::new(right, top),
        OverlayPosition::BottomLeft => PhysicalPosition::new(left, bottom),
        OverlayPosition::BottomRight => PhysicalPosition::new(right, bottom),
    };

    overlay
        .set_position(next_position)
        .map_err(|error| error.to_string())
}
