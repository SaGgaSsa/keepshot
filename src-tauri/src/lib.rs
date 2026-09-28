#[macro_use]
mod applog;
mod capture;
mod frames;
mod history;
mod output;
mod overlay;
mod platform;
mod settings;
mod tray;

use std::fs;
use std::time::Instant;
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(frames::FrameStore::default())
        .manage(overlay::OverlayRegistry::default())
        .manage(settings::SettingsState::default())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_settings(app);
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .register_asynchronous_uri_scheme_protocol("history", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let entry_id = request
                .uri()
                .path()
                .trim_start_matches('/')
                .strip_suffix("/thumb.png")
                .map(str::to_string);
            let body = entry_id
                .zip(app.path().app_local_data_dir().ok())
                .and_then(|(id, root)| history::thumbnail_path(&root.join("history"), &id))
                .and_then(|path| fs::read(path).ok())
                .unwrap_or_default();
            let status = if body.is_empty() { 404 } else { 200 };
            responder.respond(
                tauri::http::Response::builder()
                    .status(status)
                    .header(tauri::http::header::CONTENT_TYPE, "image/png")
                    .header("Cache-Control", "no-store")
                    .header(tauri::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .body(body)
                    .unwrap(),
            );
        })
        .register_asynchronous_uri_scheme_protocol("frame", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let label = request.uri().path().trim_start_matches('/').to_string();
            let query = request.uri().query().unwrap_or_default();
            let param = |name: &str| {
                query
                    .split('&')
                    .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
            };
            let rows = param("y0")
                .zip(param("y1"))
                .and_then(|(start, end)| Some((start.parse().ok()?, end.parse().ok()?)));
            let body = param("s")
                .zip(rows)
                .and_then(|(session, (start_row, end_row))| {
                    app.state::<frames::FrameStore>()
                        .frame_rows(&label, session, start_row, end_row)
                })
                .unwrap_or_default();
            let status = if body.is_empty() { 404 } else { 200 };
            responder.respond(
                tauri::http::Response::builder()
                    .status(status)
                    .header(
                        tauri::http::header::CONTENT_TYPE,
                        "application/octet-stream",
                    )
                    .header("Cache-Control", "no-store")
                    // The overlay fetch()es frames cross-origin (app origin -> frame.localhost).
                    .header(tauri::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .body(body)
                    .unwrap(),
            );
        })
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let state = app.state::<settings::SettingsState>();
                    let suspended = state.suspended.lock().map(|value| *value).unwrap_or(true);
                    if suspended {
                        return;
                    }
                    let capture = state.capture.lock().ok().and_then(|value| *value);
                    let history = state.history.lock().ok().and_then(|value| *value);
                    if capture.as_ref() == Some(shortcut) {
                        start_capture(app.clone());
                    } else if history.as_ref() == Some(shortcut) {
                        if let Err(error) = overlay::toggle_history(app) {
                            crate::log_error!("Could not toggle history panel: {error}");
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            log_client_error,
            overlay_ready,
            pending_frame,
            close_overlays,
            copy_selection,
            save_selection,
            cursor_position,
            history_list,
            history_copy,
            history_save,
            history_delete,
            history_edit,
            history_close,
            get_settings_view,
            set_shortcut,
            suspend_shortcuts,
            set_autostart,
            pick_save_folder,
            reset_save_folder,
            open_save_folder,
            open_keyboard_settings,
            check_print_screen,
            complete_onboarding
        ])
        .setup(|app| {
            applog::init(app.handle());
            initialize_settings(app.handle());
            if let Err(error) = tray::build(app.handle(), start_capture) {
                crate::log_error!("Could not initialize system tray: {error}");
            }
            if let Some(window) = app.get_webview_window("main") {
                if platform::supports_mica() {
                    let effects = tauri::utils::config::WindowEffectsConfig {
                        effects: vec![tauri::window::Effect::Mica],
                        state: None,
                        radius: None,
                        color: None,
                        ..Default::default()
                    };
                    match window.set_effects(effects) {
                        Ok(()) => {
                            if let Ok(mut material) = app
                                .state::<settings::SettingsState>()
                                .window_material
                                .lock()
                            {
                                *material = "mica".to_string();
                            }
                        }
                        Err(error) => {
                            crate::log_error!("Could not enable Mica for Settings: {error}")
                        }
                    }
                }
                let settings_window = window.clone();
                let settings_app = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Err(error) = set_shortcuts_suspended(&settings_app, false) {
                            crate::log_error!("Could not resume shortcuts: {error}");
                        }
                        if let Err(error) = settings_window.hide() {
                            crate::log_error!("Could not hide settings window: {error}");
                        }
                    }
                });
            }
            let autostart = std::env::args().any(|argument| argument == "--autostart");
            let state = app.state::<settings::SettingsState>();
            let onboarding_done = state
                .values
                .lock()
                .map(|value| value.onboarding_done)
                .unwrap_or(true);
            if !autostart && !onboarding_done {
                tray::show_settings(app.handle());
            }
            match app.path().app_local_data_dir() {
                Ok(root) => {
                    if let Err(error) = history::prepare(&root.join("history")) {
                        crate::log_error!("Could not prepare history folder: {error}");
                    }
                }
                Err(error) => crate::log_error!("Could not resolve history folder: {error}"),
            }
            if let Err(error) = overlay::create_history_window(app.handle()) {
                crate::log_error!("Could not pre-create history panel: {error}");
            }
            match capture::monitor_infos().and_then(|monitors| {
                let bounds = capture::virtual_bounds(&monitors)?;
                overlay::reconcile(app.handle(), &monitors, bounds).map(|_| ())
            }) {
                Ok(()) => {}
                Err(error) => crate::log_error!("Failed to initialize capture overlays: {error}"),
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Errors shown to the user in a webview are forwarded here so they also reach the log file.
#[tauri::command]
fn log_client_error(source: String, message: String) {
    log_error!("[{source}] {message}");
}

#[tauri::command]
fn pending_frame(state: tauri::State<'_, frames::FrameStore>) -> Option<frames::PendingFrames> {
    state.pending()
}

#[tauri::command]
fn overlay_ready(
    app: tauri::AppHandle,
    session: String,
    timings: Vec<frames::FrontendImageTiming>,
    state: tauri::State<'_, frames::FrameStore>,
) -> Result<(), String> {
    state.validate_ready(&session, &timings)?;
    let window = app
        .get_webview_window("overlay")
        .ok_or_else(|| "Capture overlay no longer exists".to_string())?;
    window
        .show()
        .map_err(|error| format!("Could not show capture overlay: {error}"))?;
    if !matches!(state.pending(), Some(current) if current.session == session) {
        if let Err(error) = window.hide() {
            crate::log_error!("Could not hide stale capture overlay: {error}");
        }
        return Ok(());
    }
    let shown_ms = state.elapsed_ms()?;
    if let Err(error) = window.set_focus() {
        crate::log_error!("Could not focus capture overlay: {error}");
    }
    let Some(metrics) = state.mark_ready(&session, timings, shown_ms)? else {
        return Ok(());
    };
    println!(
        "Capture metrics: {}",
        serde_json::to_string(&metrics).unwrap_or_default()
    );
    if let Err(error) = app.emit_to("main", "capture:metrics", metrics) {
        crate::log_error!("Failed to emit capture metrics: {error}");
    }
    Ok(())
}

#[tauri::command]
fn close_overlays(
    app: tauri::AppHandle,
    state: tauri::State<'_, frames::FrameStore>,
) -> Result<(), String> {
    close_overlay_session(&app, &state)
}

#[tauri::command]
async fn copy_selection(
    app: tauri::AppHandle,
    request: tauri::ipc::Request<'_>,
    state: tauri::State<'_, frames::FrameStore>,
) -> Result<(), String> {
    let payload = selection_request_parts(&request)?;
    state.validate_history_target(payload.history_id.as_deref())?;
    let (session, bounds, frames) = state.snapshot()?;
    let rect = payload.rect;
    let history_edit = payload.history_id.is_some();
    let composite = payload.composite.clone();
    let compose_frames = frames.clone();
    let image = tauri::async_runtime::spawn_blocking(move || {
        if history_edit {
            output::compose_history_region(bounds, compose_frames, rect, composite.as_deref())
        } else {
            output::compose(bounds, compose_frames, rect, composite.as_deref())
        }
    })
    .await
    .map_err(|error| format!("Image composition task failed: {error}"))??;
    let clipboard_image = tauri::image::Image::new(image.as_raw(), image.width(), image.height());
    app.clipboard()
        .write_image(&clipboard_image)
        .map_err(|error| format!("Could not copy selection: {error}"))?;
    start_history_record(app.clone(), bounds, frames, payload, image);
    close_overlay_session_for(&app, &app.state::<frames::FrameStore>(), &session)
}

#[tauri::command]
async fn save_selection(
    app: tauri::AppHandle,
    request: tauri::ipc::Request<'_>,
    state: tauri::State<'_, frames::FrameStore>,
) -> Result<String, String> {
    let payload = selection_request_parts(&request)?;
    state.validate_history_target(payload.history_id.as_deref())?;
    let (session, bounds, frames) = state.snapshot()?;
    let directory = save_folder(&app)?;
    let rect = payload.rect;
    let history_edit = payload.history_id.is_some();
    let composite = payload.composite.clone();
    let save_frames = frames.clone();
    let (saved_path, image) = tauri::async_runtime::spawn_blocking(move || {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Could not create {}: {error}", directory.display()))?;
        let image = if history_edit {
            output::compose_history_region(bounds, save_frames, rect, composite.as_deref())?
        } else {
            output::compose(bounds, save_frames, rect, composite.as_deref())?
        };
        let filename = format!(
            "KeepShot_{}.png",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
        );
        let path = output::unique_capture_path(&directory, &filename);
        output::save_png(&path, &image)?;
        Ok::<_, String>((path.display().to_string(), image))
    })
    .await
    .map_err(|error| format!("Image save task failed: {error}"))??;
    start_history_record(app.clone(), bounds, frames, payload, image);
    close_overlay_session_for(&app, &app.state::<frames::FrameStore>(), &session)?;
    Ok(saved_path)
}

fn selection_request_parts(
    request: &tauri::ipc::Request<'_>,
) -> Result<output::ExportPayload, String> {
    let bytes = match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes) => bytes.as_slice(),
        _ => return Err("Export body must be binary".to_string()),
    };
    output::parse_export_body(bytes)
}

fn start_history_record(
    app: tauri::AppHandle,
    bounds: capture::VirtualBounds,
    frames: Vec<frames::FrameEntry>,
    payload: output::ExportPayload,
    final_image: image::RgbaImage,
) {
    let root = match app.path().app_local_data_dir() {
        Ok(path) => path.join("history"),
        Err(error) => {
            crate::log_error!("Could not resolve history folder: {error}");
            return;
        }
    };
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| {
            let history_edit = payload.history_id.is_some();
            let compose_base = |frames| {
                if history_edit {
                    output::compose_history_region(bounds, frames, payload.rect, None)
                } else {
                    output::compose(bounds, frames, payload.rect, None)
                }
            };
            let base = compose_base(frames.clone())?;
            let redact_base = payload
                .redact_base
                .as_deref()
                .map(|bytes| {
                    if history_edit {
                        output::compose_history_region(bounds, frames, payload.rect, Some(bytes))
                    } else {
                        output::compose(bounds, frames, payload.rect, Some(bytes))
                    }
                })
                .transpose()?;
            history::record(
                &root,
                payload.rect,
                &base,
                &final_image,
                redact_base.as_ref(),
                &payload.document,
                payload.history_id.as_deref(),
            )
        })();
        if let Err(error) = result {
            crate::log_error!("Could not record capture history: {error}");
        }
    });
}

#[derive(serde::Serialize)]
struct CursorPosition {
    x: f64,
    y: f64,
}

#[tauri::command]
fn cursor_position(app: tauri::AppHandle) -> Result<CursorPosition, String> {
    let position = app
        .cursor_position()
        .map_err(|error| format!("Could not read cursor position: {error}"))?;
    Ok(CursorPosition {
        x: position.x,
        y: position.y,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsView {
    settings: settings::Settings,
    autostart: bool,
    shortcut_errors: Vec<String>,
    print_screen_conflict: Option<bool>,
    default_save_folder: String,
    window_material: String,
}

#[tauri::command]
fn get_settings_view(app: tauri::AppHandle) -> Result<SettingsView, String> {
    let current = settings::snapshot(&app)?;
    let state = app.state::<settings::SettingsState>();
    let shortcut_errors = state
        .shortcut_errors
        .lock()
        .map_err(|_| "Shortcut status is unavailable".to_string())?
        .clone();
    let window_material = state
        .window_material
        .lock()
        .map_err(|_| "Window material status is unavailable".to_string())?
        .clone();
    let autostart = app
        .autolaunch()
        .is_enabled()
        .map_err(|error| format!("Could not read startup setting: {error}"))?;
    let print_screen_conflict = if current
        .capture_shortcut
        .to_ascii_lowercase()
        .contains("printscreen")
    {
        platform::snipping_tool_owns_print_screen()
    } else {
        None
    };
    Ok(SettingsView {
        settings: current,
        autostart,
        shortcut_errors,
        print_screen_conflict,
        default_save_folder: default_save_folder(&app)?.display().to_string(),
        window_material,
    })
}

#[tauri::command]
fn set_shortcut(
    app: tauri::AppHandle,
    kind: String,
    value: String,
) -> Result<settings::Settings, String> {
    let mut current = settings::snapshot(&app)?;
    let parsed = settings::shortcut(&value)?;
    let (old_value, slot) = match kind.as_str() {
        "capture" => (current.capture_shortcut.clone(), "capture"),
        "history" => (current.history_shortcut.clone(), "history"),
        _ => return Err("Shortcut kind must be capture or history".to_string()),
    };
    let other_value = if slot == "capture" {
        &current.history_shortcut
    } else {
        &current.capture_shortcut
    };
    if settings::shortcut(other_value)? == parsed {
        return Err("Capture and history shortcuts must be different".to_string());
    }
    let manager = app.global_shortcut();
    let is_suspended = app
        .state::<settings::SettingsState>()
        .suspended
        .lock()
        .map_err(|_| "Shortcut status is unavailable".to_string())?
        .to_owned();
    if !is_suspended {
        if let Ok(old) = settings::shortcut(&old_value) {
            let _ = manager.unregister(old);
        }
    }
    if !is_suspended {
        if let Err(error) = manager.register(parsed) {
            if let Ok(old) = settings::shortcut(&old_value) {
                if let Err(restore_error) = manager.register(old) {
                    crate::log_error!("Could not restore previous shortcut: {restore_error}");
                } else {
                    let state = app.state::<settings::SettingsState>();
                    set_shortcut_slot(&state, slot, Some(old));
                }
            }
            return Err(settings::registration_error(&error.to_string()));
        }
    }
    if slot == "capture" {
        current.capture_shortcut = value;
    } else {
        current.history_shortcut = value;
    }
    if let Err(error) = settings::update(&app, current.clone()) {
        if !is_suspended {
            let _ = manager.unregister(parsed);
            if let Ok(old) = settings::shortcut(&old_value) {
                let _ = manager.register(old);
                set_shortcut_slot(&app.state::<settings::SettingsState>(), slot, Some(old));
            }
        }
        return Err(error);
    }
    let updated = settings::shortcut(if slot == "capture" {
        &current.capture_shortcut
    } else {
        &current.history_shortcut
    })?;
    set_shortcut_slot(&app.state::<settings::SettingsState>(), slot, Some(updated));
    if let Ok(mut errors) = app
        .state::<settings::SettingsState>()
        .shortcut_errors
        .lock()
    {
        errors.retain(|error| !error.starts_with(slot));
    }
    if let Err(error) = tray::update_menu(&app) {
        crate::log_error!("Could not refresh tray menu: {error}");
    }
    Ok(current)
}

fn set_shortcut_slot(state: &settings::SettingsState, slot: &str, shortcut: Option<Shortcut>) {
    let target = if slot == "capture" {
        &state.capture
    } else {
        &state.history
    };
    if let Ok(mut value) = target.lock() {
        *value = shortcut;
    }
}

#[tauri::command]
fn suspend_shortcuts(app: tauri::AppHandle, suspend: bool) -> Result<(), String> {
    set_shortcuts_suspended(&app, suspend)
}

fn set_shortcuts_suspended(app: &tauri::AppHandle, suspend: bool) -> Result<(), String> {
    let state = app.state::<settings::SettingsState>();
    let mut suspended = state
        .suspended
        .lock()
        .map_err(|_| "Shortcut status is unavailable".to_string())?;
    if *suspended == suspend {
        return Ok(());
    }
    let current = settings::snapshot(app)?;
    let manager = app.global_shortcut();
    let values = [current.capture_shortcut, current.history_shortcut];
    if suspend {
        for value in values {
            if let Ok(shortcut) = settings::shortcut(&value) {
                let _ = manager.unregister(shortcut);
            }
        }
    } else {
        let mut registered = Vec::new();
        for value in values {
            let shortcut = settings::shortcut(&value)?;
            if let Err(error) = manager.register(shortcut) {
                for previous in registered {
                    let _ = manager.unregister(previous);
                }
                return Err(settings::registration_error(&error.to_string()));
            }
            registered.push(shortcut);
        }
    }
    *suspended = suspend;
    Ok(())
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|error| error.to_string())
    } else {
        manager.disable().map_err(|error| error.to_string())
    }
}

#[tauri::command]
async fn pick_save_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let dialog_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        dialog_app.dialog().file().blocking_pick_folder()
    })
    .await
    .map_err(|error| format!("Folder picker failed: {error}"))?;
    let Some(path) = selected else {
        return Ok(None);
    };
    let folder = path
        .into_path()
        .map_err(|error| format!("Invalid folder: {error}"))?;
    let mut current = settings::snapshot(&app)?;
    current.save_folder = Some(folder.display().to_string());
    settings::update(&app, current)?;
    Ok(Some(folder.display().to_string()))
}

#[tauri::command]
fn reset_save_folder(app: tauri::AppHandle) -> Result<(), String> {
    let mut current = settings::snapshot(&app)?;
    current.save_folder = None;
    settings::update(&app, current)
}

#[tauri::command]
fn open_save_folder(app: tauri::AppHandle) -> Result<(), String> {
    let folder = save_folder(&app)?;
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    platform::open_folder(&folder)
}

#[tauri::command]
fn open_keyboard_settings() -> Result<(), String> {
    platform::open_keyboard_settings()
}

#[tauri::command]
fn check_print_screen(app: tauri::AppHandle) -> Option<bool> {
    let settings = settings::snapshot(&app).ok()?;
    if settings
        .capture_shortcut
        .to_ascii_lowercase()
        .contains("printscreen")
    {
        platform::snipping_tool_owns_print_screen()
    } else {
        None
    }
}

#[tauri::command]
fn complete_onboarding(app: tauri::AppHandle) -> Result<(), String> {
    let mut current = settings::snapshot(&app)?;
    current.onboarding_done = true;
    settings::update(&app, current)
}

fn default_save_folder(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .picture_dir()
        .map_err(|error| format!("Could not find the Pictures folder: {error}"))?
        .join("KeepShot"))
}

fn save_folder(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    match settings::snapshot(app)?.save_folder {
        Some(folder) => Ok(std::path::PathBuf::from(folder)),
        None => default_save_folder(app),
    }
}

fn initialize_settings(app: &tauri::AppHandle) {
    let initial = match settings::load(app) {
        Ok(value) => value,
        Err(error) => {
            crate::log_error!("Could not load settings; defaults will be used: {error}");
            settings::Settings::default()
        }
    };
    if let Ok(mut current) = app.state::<settings::SettingsState>().values.lock() {
        *current = initial;
    }
    settings::register_startup_shortcuts(app);
}

#[tauri::command]
fn history_list(app: tauri::AppHandle) -> Result<Vec<history::HistoryItem>, String> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("history");
    history::list(&root)
}

#[tauri::command]
fn history_copy(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("history");
    let image = history::load_final(&root, &id)?;
    let clipboard_image = tauri::image::Image::new(image.as_raw(), image.width(), image.height());
    app.clipboard()
        .write_image(&clipboard_image)
        .map_err(|error| format!("Could not copy history image: {error}"))
}

#[tauri::command]
async fn history_save(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("history");
    let image = history::load_final(&root, &id)?;
    let directory = save_folder(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Could not create {}: {error}", directory.display()))?;
        let filename = format!(
            "KeepShot_{}.png",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
        );
        let path = output::unique_capture_path(&directory, &filename);
        output::save_png(&path, &image)?;
        Ok(path.display().to_string())
    })
    .await
    .map_err(|error| format!("History image save task failed: {error}"))?
}

#[tauri::command]
fn history_delete(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("history");
    history::delete(&root, &id)
}

#[tauri::command]
fn history_close(app: tauri::AppHandle) -> Result<(), String> {
    overlay::hide_history(&app)
}

#[tauri::command]
fn history_edit(
    app: tauri::AppHandle,
    id: String,
    state: tauri::State<'_, frames::FrameStore>,
) -> Result<(), String> {
    if !state.try_start() {
        return Err("Another capture session is active".to_string());
    }
    let result = (|| {
        let root = app
            .path()
            .app_local_data_dir()
            .map_err(|error| error.to_string())?
            .join("history");
        let capture = history::load_edit(&root, &id)?;
        let monitors = capture::monitor_infos()?;
        let bounds = capture::virtual_bounds(&monitors)?;
        overlay::reconcile(&app, &monitors, bounds)?;
        let cursor = app.cursor_position().map_err(|error| error.to_string())?;
        let monitor = monitors
            .iter()
            .find(|monitor| {
                cursor.x >= f64::from(monitor.x)
                    && cursor.x < f64::from(monitor.x) + f64::from(monitor.width)
                    && cursor.y >= f64::from(monitor.y)
                    && cursor.y < f64::from(monitor.y) + f64::from(monitor.height)
            })
            .or_else(|| monitors.first())
            .ok_or_else(|| "No monitor is available for editing".to_string())?;
        let width = capture.original.width();
        let height = capture.original.height();
        let (x, y) = if width <= monitor.width && height <= monitor.height {
            (
                monitor.x + ((monitor.width - width) / 2) as i32,
                monitor.y + ((monitor.height - height) / 2) as i32,
            )
        } else {
            (monitor.x, monitor.y)
        };
        let rect = output::SelectionRect {
            x,
            y,
            width,
            height,
        };
        let frame = frames::FrameEntry::new(capture::CapturedFrame {
            monitor: capture::MonitorInfo {
                label: format!("history-{}", capture.meta.id),
                name: "History capture".to_string(),
                x,
                y,
                width,
                height,
                scale_factor: monitor.scale_factor,
            },
            bytes: capture.original.into_raw(),
            capture_ms: 0.0,
            prepare_ms: 0.0,
        });
        let session = frames::next_session_id();
        state.begin_edit(
            session.clone(),
            Instant::now(),
            bounds,
            vec![frame],
            frames::EditFrameInfo {
                history_id: capture.meta.id,
                rect,
                document: capture.document,
            },
        )?;
        let pending = state
            .pending()
            .ok_or_else(|| "Edited capture session could not be created".to_string())?;
        overlay::hide_history(&app)?;
        app.emit_to("overlay", "overlay:frame", pending)
            .map_err(|error| format!("Could not open history capture in editor: {error}"))
    })();
    if result.is_err() {
        let _ = state.clear();
    }
    result
}

fn close_overlay_session(app: &tauri::AppHandle, state: &frames::FrameStore) -> Result<(), String> {
    state.clear()?;
    notify_overlay_closed(app);
    Ok(())
}

fn close_overlay_session_for(
    app: &tauri::AppHandle,
    state: &frames::FrameStore,
    session: &str,
) -> Result<(), String> {
    if state.clear_if_session(session)? {
        notify_overlay_closed(app);
    }
    Ok(())
}

fn notify_overlay_closed(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        if let Err(error) = window.emit("overlay:clear", ()) {
            crate::log_error!(
                "Failed to notify capture overlay that its frame was cleared: {error}"
            );
        }
        if let Err(error) = window.hide() {
            crate::log_error!("Failed to hide capture overlay: {error}");
        }
    }
}

fn start_capture(app: tauri::AppHandle) {
    let store = app.state::<frames::FrameStore>();
    if !store.try_start() {
        // A session whose overlays never became visible (e.g. a frame failed to
        // decode) leaves no window to press Esc on, so recover here instead.
        if any_overlay_visible(&app) {
            return;
        }
        let _ = store.clear();
        hide_all_overlays(&app);
        if !store.try_start() {
            return;
        }
    }
    let started = Instant::now();
    tauri::async_runtime::spawn(async move {
        let app_for_capture = app.clone();
        let result =
            tauri::async_runtime::spawn_blocking(move || capture_session(app_for_capture, started))
                .await;
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                crate::log_error!("Capture failed: {error}");
                let _ = app.state::<frames::FrameStore>().clear();
                hide_all_overlays(&app);
            }
            Err(error) => {
                crate::log_error!("Capture task failed: {error}");
                let _ = app.state::<frames::FrameStore>().clear();
                hide_all_overlays(&app);
            }
        }
    });
}

fn capture_session(app: tauri::AppHandle, started: Instant) -> Result<(), String> {
    let infos = capture::monitor_infos()?;
    let bounds = capture::virtual_bounds(&infos)?;
    overlay::reconcile(&app, &infos, bounds)?;
    let capture_started = Instant::now();
    let captured = capture::capture_all(&infos)?;
    let capture_total_ms = capture_started.elapsed().as_secs_f64() * 1000.0;
    let session = frames::next_session_id();
    let pending: Vec<_> = captured.into_iter().map(frames::FrameEntry::new).collect();
    app.state::<frames::FrameStore>().begin(
        session.clone(),
        started,
        capture_total_ms,
        bounds,
        pending,
    )?;
    let store = app.state::<frames::FrameStore>();
    let payload = store
        .pending()
        .ok_or_else(|| "Captured frame session was not available".to_string())?;
    if let Err(error) = store.set_emit_time() {
        crate::log_error!("Could not record overlay event time: {error}");
    }
    if let Err(error) = app.emit_to("overlay", "overlay:frame", payload) {
        crate::log_error!("Failed to notify capture overlay about the new session: {error}");
    }
    Ok(())
}

fn any_overlay_visible(app: &tauri::AppHandle) -> bool {
    app.get_webview_window("overlay")
        .is_some_and(|window| window.is_visible().unwrap_or(false))
}

fn hide_all_overlays(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        if let Err(error) = window.emit("overlay:clear", ()) {
            crate::log_error!("Failed to clear capture overlay contents: {error}");
        }
        if let Err(error) = window.hide() {
            crate::log_error!("Failed to hide capture overlay: {error}");
        }
    }
}
