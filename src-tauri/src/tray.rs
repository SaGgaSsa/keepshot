use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::{overlay, settings, updates};

/// A simplified two-layer mark that stays legible at 16-20 px, unlike the full app icon.
fn tray_icon() -> Result<Image<'static>, String> {
    let decoded = image::load_from_memory(include_bytes!("../icons/tray-32.png"))
        .map_err(|error| format!("Could not decode the tray icon: {error}"))?
        .into_rgba8();
    let (width, height) = decoded.dimensions();
    Ok(Image::new_owned(decoded.into_raw(), width, height))
}

pub fn build(app: &AppHandle, start_capture: fn(AppHandle)) -> Result<(), String> {
    let icon = tray_icon()?;
    let menu = make_menu(app)?;
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("KeepShot")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                start_capture(tray.app_handle().clone());
            }
        })
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "capture" => start_capture(app.clone()),
            "history" => {
                if let Err(error) = overlay::toggle_history(app) {
                    crate::log_error!("Could not toggle history panel: {error}");
                }
            }
            "settings" => show_settings(app),
            "install-update" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = updates::install_update(app.clone()).await {
                        crate::log_error!("Could not install update from tray: {error}");
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)
        .map_err(|error| format!("Could not create system tray icon: {error}"))?;
    Ok(())
}

pub fn update_menu(app: &AppHandle) -> Result<(), String> {
    let menu = make_menu(app)?;
    let tray = app
        .tray_by_id("main")
        .ok_or_else(|| "System tray icon is unavailable".to_string())?;
    tray.set_menu(Some(menu))
        .map_err(|error| format!("Could not update tray menu: {error}"))?;
    let tooltip = if updates::available_version(app).is_some() {
        "KeepShot — update available"
    } else {
        "KeepShot"
    };
    tray.set_tooltip(Some(tooltip))
        .map_err(|error| format!("Could not update tray tooltip: {error}"))
}

pub fn show_settings(app: &AppHandle) {
    let state = app.state::<settings::SettingsState>();
    if let Ok(ready) = state.page_ready.lock() {
        if !*ready {
            // The page calls `settings_ready` after its first paint and the window is shown then.
            if let Ok(mut pending) = state.show_pending.lock() {
                *pending = true;
            }
            return;
        }
    }
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.show() {
            crate::log_error!("Could not show settings window: {error}");
        }
        if let Err(error) = window.set_focus() {
            crate::log_error!("Could not focus settings window: {error}");
        }
    }
}

fn make_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, String> {
    let current = settings::snapshot(app)?;
    let update = updates::available_version(app)
        .map(|version| {
            MenuItem::with_id(
                app,
                "install-update",
                format!("Install update v{version}"),
                true,
                None::<&str>,
            )
            .map_err(|error| error.to_string())
        })
        .transpose()?;
    let capture = MenuItem::with_id(
        app,
        "capture",
        format!("Capture\t{}", display_shortcut(&current.capture_shortcut)),
        true,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let history = MenuItem::with_id(app, "history", "History", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "Quit KeepShot", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let first_separator = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
    let second_separator = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
    if let Some(update) = update {
        let update_separator =
            PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
        Menu::with_items(
            app,
            &[
                &update,
                &update_separator,
                &capture,
                &history,
                &first_separator,
                &settings,
                &second_separator,
                &quit,
            ],
        )
    } else {
        Menu::with_items(
            app,
            &[
                &capture,
                &history,
                &first_separator,
                &settings,
                &second_separator,
                &quit,
            ],
        )
    }
    .map_err(|error| error.to_string())
}

fn display_shortcut(shortcut: &str) -> String {
    shortcut
        .split('+')
        .map(|token| match token {
            "PrintScreen" => "Print Screen",
            "Super" => "Win",
            _ => token,
        })
        .collect::<Vec<_>>()
        .join(" + ")
}
