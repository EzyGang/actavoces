use super::settings::validate_dictation_hotkey;

#[test]
fn dictation_shortcuts_accept_single_keys_and_combinations() {
    if cfg!(target_os = "linux")
        && (std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE")
                .is_ok_and(|session| session.eq_ignore_ascii_case("wayland")))
    {
        return;
    }

    if !cfg!(target_os = "macos") {
        assert_eq!(validate_dictation_hotkey("R"), Ok(()));
    }
    assert_eq!(
        validate_dictation_hotkey("CommandOrControl+Shift+D"),
        Ok(())
    );
}

#[test]
fn dictation_shortcuts_reject_modifier_media_and_system_keys() {
    if cfg!(target_os = "linux")
        && (std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE")
                .is_ok_and(|session| session.eq_ignore_ascii_case("wayland")))
    {
        return;
    }

    for shortcut in ["Shift", "Fn", "MediaPlayPause", "BrowserBack"] {
        assert!(validate_dictation_hotkey(shortcut).is_err());
    }
}
