mod capture;
mod frames;
mod overlay;

use std::time::Instant;
use tauri::{Emitter, Manager};
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
                    .header(tauri::http::header::CONTENT_TYPE, "image/bmp")
                    .header("Cache-Control", "no-store")
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
            close_overlays
        ])
        .setup(|app| {
            app.global_shortcut().register(capture_shortcut())?;
            match capture::monitor_infos()
                .and_then(|monitors| overlay::reconcile(app.handle(), &monitors).map(|_| ()))
            {
                Ok(()) => {}
                Err(error) => eprintln!("Failed to initialize capture overlays: {error}"),
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn pending_frame(
    label: String,
    state: tauri::State<'_, frames::FrameStore>,
) -> Option<frames::PendingFrame> {
    state.pending(&label)
}

#[tauri::command]
fn overlay_ready(
    app: tauri::AppHandle,
    label: String,
    session: String,
    state: tauri::State<'_, frames::FrameStore>,
) -> Result<(), String> {
    let Some(pending) = state.pending(&label) else {
        return Ok(());
    };
    if pending.session != session {
        return Ok(());
    }
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("Overlay {label} no longer exists"))?;
    window
        .show()
        .map_err(|error| format!("Could not show {label}: {error}"))?;
    if !matches!(state.pending(&label), Some(current) if current.session == session) {
        if let Err(error) = window.hide() {
            eprintln!("Could not hide stale overlay {label}: {error}");
        }
        return Ok(());
    }
    let cursor = app.cursor_position().ok();
    let cursor_monitor = cursor.and_then(|position| {
        if position.x >= pending.monitor.x as f64
            && position.y >= pending.monitor.y as f64
            && position.x < pending.monitor.x as f64 + pending.monitor.width as f64
            && position.y < pending.monitor.y as f64 + pending.monitor.height as f64
        {
            Some(label.clone())
        } else {
            None
        }
    });
    if cursor_monitor.as_deref() == Some(label.as_str()) {
        if let Err(error) = window.set_focus() {
            eprintln!("Could not focus cursor monitor overlay {label}: {error}");
        }
    }
    let Some(metrics) = state.mark_ready(&label, &session)? else {
        return Ok(());
    };
    if cursor_monitor.is_none() {
        if let Some(target) = cursor.and_then(|position| {
            metrics.monitors.iter().find(|monitor| {
                position.x >= monitor.x as f64
                    && position.y >= monitor.y as f64
                    && position.x < monitor.x as f64 + monitor.width as f64
                    && position.y < monitor.y as f64 + monitor.height as f64
            })
        }) {
            if let Some(target_window) = app.get_webview_window(&target.label) {
                if let Err(error) = target_window.set_focus() {
                    eprintln!(
                        "Could not focus cursor monitor overlay {}: {error}",
                        target.label
                    );
                }
            }
        }
    }
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
    state.clear()?;
    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-") {
            if let Err(error) = window.emit("overlay:clear", ()) {
                eprintln!("Failed to notify {label} that its frame was cleared: {error}");
            }
            if let Err(error) = window.hide() {
                eprintln!("Failed to hide {label}: {error}");
            }
        }
    }
    Ok(())
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
    let reconcile_started = Instant::now();
    overlay::reconcile(&app, &infos)?;
    let reconciliation_ms = reconcile_started.elapsed().as_secs_f64() * 1000.0;
    let capture_started = Instant::now();
    let captured = capture::capture_all()?;
    let capture_total_ms = capture_started.elapsed().as_secs_f64() * 1000.0;
    let session = frames::next_session_id();
    let pending: Vec<_> = captured.into_iter().map(frames::FrameEntry::new).collect();
    app.state::<frames::FrameStore>().begin(
        session.clone(),
        started,
        reconciliation_ms,
        capture_total_ms,
        pending.clone(),
    )?;
    for frame in pending {
        let label = frame.monitor.label.clone();
        if let Err(error) = app.emit_to(
            &label,
            "overlay:frame",
            frames::PendingFrame {
                session: session.clone(),
                monitor: frame.monitor,
            },
        ) {
            eprintln!("Failed to notify {label} about its capture frame: {error}");
        }
    }
    let _ = app.emit_to("main", "capture:started", session);
    Ok(())
}

fn any_overlay_visible(app: &tauri::AppHandle) -> bool {
    app.webview_windows().iter().any(|(label, window)| {
        label.starts_with("overlay-") && window.is_visible().unwrap_or(false)
    })
}

fn hide_all_overlays(app: &tauri::AppHandle) {
    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-") {
            if let Err(error) = window.hide() {
                eprintln!("Failed to hide {label}: {error}");
            }
        }
    }
}
