use std::sync::Mutex;
use std::time::Instant;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};

use crate::capture::{MonitorInfo, VirtualBounds};

#[derive(Default)]
pub struct OverlayRegistry {
    topology: Mutex<Option<(Vec<MonitorInfo>, VirtualBounds)>>,
}

pub fn reconcile(
    app: &AppHandle,
    monitors: &[MonitorInfo],
    bounds: VirtualBounds,
) -> Result<f64, String> {
    let started = Instant::now();
    let registry = app.state::<OverlayRegistry>();
    let mut known = registry
        .topology
        .lock()
        .map_err(|_| "Overlay registry is unavailable".to_string())?;
    if app.get_webview_window("overlay").is_some()
        && known.as_ref().is_some_and(|(previous, previous_bounds)| {
            *previous_bounds == bounds && same_topology(previous, monitors)
        })
    {
        return Ok(started.elapsed().as_secs_f64() * 1000.0);
    }

    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-") {
            window
                .close()
                .map_err(|error| format!("Could not close stale overlay {label}: {error}"))?;
        }
    }
    let window = match app.get_webview_window("overlay") {
        Some(window) => window,
        None => create_overlay(app)?,
    };
    position_window(&window, bounds)?;
    *known = Some((monitors.to_vec(), bounds));
    Ok(started.elapsed().as_secs_f64() * 1000.0)
}

pub fn create_history_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    let window = WebviewWindowBuilder::new(app, "history", WebviewUrl::App("history".into()))
        .title("KeepShot History")
        .inner_size(420.0, 560.0)
        .transparent(true)
        .shadow(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|error| format!("Could not create history panel: {error}"))?;
    let app = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false)) {
            if let Some(window) = app.get_webview_window("history") {
                if let Err(error) = window.hide() {
                    crate::log_error!("Could not hide history panel after focus loss: {error}");
                }
            }
        }
    });
    Ok(window)
}

pub fn toggle_history(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("history")
        .ok_or_else(|| "History panel is unavailable".to_string())?;
    if window.is_visible().map_err(|error| error.to_string())? {
        return window.hide().map_err(|error| error.to_string());
    }
    position_history(&window, app)?;
    window.show().map_err(|error| error.to_string())?;
    if let Err(error) = focus(&window) {
        crate::log_error!("Could not focus history panel: {error}");
    }
    if let Err(error) = window.emit("history:shown", ()) {
        crate::log_error!("Could not notify history panel that it was shown: {error}");
    }
    Ok(())
}

/// Focuses a window created with `focused(false)`. On Linux that flag also keeps the webview from
/// taking GTK keyboard focus, so keys (Escape included) never reach the page unless it is focused too.
pub fn focus(window: &WebviewWindow) -> Result<(), String> {
    window.set_focus().map_err(|error| error.to_string())?;
    #[cfg(target_os = "linux")]
    AsRef::<tauri::Webview>::as_ref(window)
        .set_focus()
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Linux window managers (GNOME/Mutter included) clamp normal windows to the work area, which
/// shifts the overlay past the top bar and dock. A fullscreen window spanning every monitor is
/// exempt from that and stacks above the panels. Must run on the main (GTK) thread.
#[cfg(target_os = "linux")]
pub fn cover_all_monitors(window: &WebviewWindow) -> Result<(), String> {
    use gtk::prelude::*;
    let gtk_window = window.gtk_window().map_err(|error| error.to_string())?;
    gtk_window.realize();
    let gdk_window = gtk_window
        .window()
        .ok_or_else(|| "Capture overlay has no GDK window".to_string())?;
    gdk_window.set_fullscreen_mode(gtk::gdk::FullscreenMode::AllMonitors);
    gtk_window.fullscreen();
    Ok(())
}

pub fn hide_history(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("history") {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn position_history(window: &WebviewWindow, app: &AppHandle) -> Result<(), String> {
    let cursor = app.cursor_position().map_err(|error| error.to_string())?;
    let monitors = crate::capture::monitor_infos()?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            cursor.x >= f64::from(monitor.x)
                && cursor.x < f64::from(monitor.x) + f64::from(monitor.width)
                && cursor.y >= f64::from(monitor.y)
                && cursor.y < f64::from(monitor.y) + f64::from(monitor.height)
        })
        .or_else(|| monitors.first())
        .ok_or_else(|| "No monitor is available for the history panel".to_string())?;
    let scale = f64::from(monitor.scale_factor.max(1.0));
    let panel_width = (420.0 * scale).round() as i32;
    let panel_height = (560.0 * scale).round() as i32;
    let margin = (16.0 * scale).round() as i32;
    let taskbar = (48.0 * scale).round() as i32;
    let left = monitor.x + monitor.width as i32 - panel_width - margin;
    let top = monitor.y + monitor.height as i32 - panel_height - margin - taskbar;
    let x = left.max(monitor.x);
    let y = top.max(monitor.y);
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())
}

fn same_topology(left: &[MonitorInfo], right: &[MonitorInfo]) -> bool {
    left.len() == right.len()
        && left.iter().all(|previous| {
            right.iter().any(|current| {
                previous.label == current.label
                    && previous.x == current.x
                    && previous.y == current.y
                    && previous.width == current.width
                    && previous.height == current.height
                    && previous.scale_factor == current.scale_factor
            })
        })
}

fn create_overlay(app: &AppHandle) -> Result<WebviewWindow, String> {
    WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay".into()))
        .title("KeepShot Capture")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|error| format!("Could not create capture overlay: {error}"))
}

fn position_window(window: &WebviewWindow, bounds: VirtualBounds) -> Result<(), String> {
    let position = PhysicalPosition::new(bounds.x, bounds.y);
    let size = PhysicalSize::new(bounds.width, bounds.height);
    window
        .set_position(position)
        .map_err(|error| error.to_string())?;
    window.set_size(size).map_err(|error| error.to_string())?;
    window
        .set_position(position)
        .map_err(|error| error.to_string())?;
    let actual_position = window.outer_position().map_err(|error| error.to_string())?;
    let actual_size = window.outer_size().map_err(|error| error.to_string())?;
    if actual_position != position || actual_size != size {
        crate::log_error!(
            "Overlay geometry mismatch: wanted ({}, {}) {}x{}, got ({}, {}) {}x{}",
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            actual_position.x,
            actual_position.y,
            actual_size.width,
            actual_size.height,
        );
    }
    Ok(())
}
