mod capture;
mod frames;
mod history;
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

fn history_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL), Code::PrintScreen)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(frames::FrameStore::default())
        .manage(overlay::OverlayRegistry::default())
        .plugin(tauri_plugin_clipboard_manager::init())
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
                    if shortcut == &capture_shortcut() && event.state() == ShortcutState::Pressed {
                        start_capture(app.clone());
                    } else if shortcut == &history_shortcut()
                        && event.state() == ShortcutState::Pressed
                    {
                        if let Err(error) = overlay::toggle_history(app) {
                            eprintln!("Could not toggle history panel: {error}");
                        }
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
            cursor_position,
            history_list,
            history_copy,
            history_save,
            history_delete,
            history_edit,
            history_close
        ])
        .setup(|app| {
            app.global_shortcut().register(capture_shortcut())?;
            if let Err(error) = app.global_shortcut().register(history_shortcut()) {
                eprintln!("Could not register history shortcut: {error}");
            }
            match app.path().app_local_data_dir() {
                Ok(root) => {
                    if let Err(error) = history::prepare(&root.join("history")) {
                        eprintln!("Could not prepare history folder: {error}");
                    }
                }
                Err(error) => eprintln!("Could not resolve history folder: {error}"),
            }
            if let Err(error) = overlay::create_history_window(app.handle()) {
                eprintln!("Could not pre-create history panel: {error}");
            }
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
    let directory = app
        .path()
        .picture_dir()
        .map_err(|error| format!("Could not find the Pictures folder: {error}"))?
        .join("KeepShot");
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
            eprintln!("Could not resolve history folder: {error}");
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
            eprintln!("Could not record capture history: {error}");
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
    let directory = app
        .path()
        .picture_dir()
        .map_err(|error| format!("Could not find the Pictures folder: {error}"))?
        .join("KeepShot");
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
