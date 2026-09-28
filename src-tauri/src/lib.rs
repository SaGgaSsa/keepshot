mod capture;
mod frames;
mod output;
mod overlay;

use std::fs;
use std::time::Instant;
use tauri::{Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const CAPTURE_SHORTCUT_MODIFIERS: Modifiers = Modifiers::CONTROL.union(Modifiers::SHIFT);
const CAPTURE_SHORTCUT_CODE: Code = Code::KeyX;

fn capture_shortcut() -> Shortcut {
    Shortcut::new(Some(CAPTURE_SHORTCUT_MODIFIERS), CAPTURE_SHORTCUT_CODE)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(frames::FrameStore::default())
        .manage(overlay::OverlayRegistry::default())
        .plugin(tauri_plugin_clipboard_manager::init())
        .register_asynchronous_uri_scheme_protocol("frame", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let label = request.uri().path().trim_start_matches('/').to_string();
            let session = request
                .uri()
                .query()
                .and_then(|query| query.split('&').find_map(|pair| pair.strip_prefix("s=")))
                .map(str::to_owned);
            let body = session
                .and_then(|session| {
                    app.state::<frames::FrameStore>()
                        .frame_bytes(&label, &session)
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
                    if shortcut == &capture_shortcut() && event.state() == ShortcutState::Pressed {
                        start_capture(app.clone());
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            overlay_ready,
            pending_frame,
            close_overlays,
            copy_selection,
            save_selection,
            cursor_position
        ])
        .setup(|app| {
            app.global_shortcut().register(capture_shortcut())?;
            match capture::monitor_infos().and_then(|monitors| {
                let bounds = capture::virtual_bounds(&monitors)?;
                overlay::reconcile(app.handle(), &monitors, bounds).map(|_| ())
            }) {
                Ok(()) => {}
                Err(error) => eprintln!("Failed to initialize capture overlays: {error}"),
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
            eprintln!("Could not hide stale capture overlay: {error}");
        }
        return Ok(());
    }
    let shown_ms = state.elapsed_ms()?;
    if let Err(error) = window.set_focus() {
        eprintln!("Could not focus capture overlay: {error}");
    }
    let Some(metrics) = state.mark_ready(&session, timings, shown_ms)? else {
        return Ok(());
    };
    println!(
        "Capture metrics: {}",
        serde_json::to_string(&metrics).unwrap_or_default()
    );
    if let Err(error) = app.emit_to("main", "capture:metrics", metrics) {
        eprintln!("Failed to emit capture metrics: {error}");
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
    rect: output::SelectionRect,
    state: tauri::State<'_, frames::FrameStore>,
) -> Result<(), String> {
    let (session, bounds, frames) = state.snapshot()?;
    let image = tauri::async_runtime::spawn_blocking(move || output::compose(bounds, frames, rect))
        .await
        .map_err(|error| format!("Image composition task failed: {error}"))??;
    let clipboard_image = tauri::image::Image::new(image.as_raw(), image.width(), image.height());
    app.clipboard()
        .write_image(&clipboard_image)
        .map_err(|error| format!("Could not copy selection: {error}"))?;
    close_overlay_session_for(&app, &app.state::<frames::FrameStore>(), &session)
}

#[tauri::command]
async fn save_selection(
    app: tauri::AppHandle,
    rect: output::SelectionRect,
    state: tauri::State<'_, frames::FrameStore>,
) -> Result<String, String> {
    let (session, bounds, frames) = state.snapshot()?;
    let directory = app
        .path()
        .picture_dir()
        .map_err(|error| format!("Could not find the Pictures folder: {error}"))?
        .join("KeepShot");
    let saved_path = tauri::async_runtime::spawn_blocking(move || {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Could not create {}: {error}", directory.display()))?;
        let image = output::compose(bounds, frames, rect)?;
        let filename = format!(
            "KeepShot_{}.png",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
        );
        let path = output::unique_capture_path(&directory, &filename);
        output::save_png(&path, &image)?;
        Ok::<_, String>(path.display().to_string())
    })
    .await
    .map_err(|error| format!("Image save task failed: {error}"))??;
    close_overlay_session_for(&app, &app.state::<frames::FrameStore>(), &session)?;
    Ok(saved_path)
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
            eprintln!("Failed to notify capture overlay that its frame was cleared: {error}");
        }
        if let Err(error) = window.hide() {
            eprintln!("Failed to hide capture overlay: {error}");
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
                eprintln!("Capture failed: {error}");
                let _ = app.state::<frames::FrameStore>().clear();
                hide_all_overlays(&app);
            }
            Err(error) => {
                eprintln!("Capture task failed: {error}");
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
        eprintln!("Could not record overlay event time: {error}");
    }
    if let Err(error) = app.emit_to("overlay", "overlay:frame", payload) {
        eprintln!("Failed to notify capture overlay about the new session: {error}");
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
            eprintln!("Failed to clear capture overlay contents: {error}");
        }
        if let Err(error) = window.hide() {
            eprintln!("Failed to hide capture overlay: {error}");
        }
    }
}
