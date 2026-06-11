use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

const TRAY_ID: &str = "actavoces-tray";

pub fn init_tray(app: &tauri::App) -> Result<(), tauri::Error> {
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_item])?;
    let _ = TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => {
                show_main_window(tray.app_handle());
            }
            _ => (),
        })
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                app.exit(0);
            }
        })
        .build(app)?;

    Ok(())
}

pub fn sync_tray_recording_icon(app: &tauri::AppHandle, recording: bool) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let icon = match recording {
        true => app.default_window_icon().map(recording_tray_icon),
        false => app.default_window_icon().cloned(),
    };
    let _ = tray.set_icon(icon);
    let _ = tray.set_tooltip(Some(match recording {
        true => "ActaVoces is recording",
        false => "ActaVoces",
    }));
}

fn show_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let _ = window.show();
    let _ = window.set_focus();
}

fn recording_tray_icon(base: &Image<'_>) -> Image<'static> {
    let width = base.width();
    let height = base.height();
    let mut rgba = base.rgba().to_vec();
    let radius = (width.min(height) / 5).max(4) as i32;
    let center_x = radius;
    let center_y = radius;

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let distance = (x - center_x).pow(2) + (y - center_y).pow(2);
            if distance > radius.pow(2) {
                continue;
            }

            let index = ((y as u32 * width + x as u32) * 4) as usize;
            let pixel = if distance >= (radius - 1).pow(2) {
                [127, 29, 29, 255]
            } else {
                [220, 38, 38, 255]
            };

            rgba[index..index + 4].copy_from_slice(&pixel);
        }
    }

    Image::new_owned(rgba, width, height)
}
