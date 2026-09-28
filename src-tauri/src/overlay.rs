use std::sync::Mutex;
use std::time::Instant;

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::capture::MonitorInfo;

#[derive(Default)]
pub struct OverlayRegistry {
    monitors: Mutex<Vec<MonitorInfo>>,
}

pub fn reconcile(app: &AppHandle, monitors: &[MonitorInfo]) -> Result<f64, String> {
    let started = Instant::now();
    let registry = app.state::<OverlayRegistry>();
    let mut known = registry
        .monitors
        .lock()
        .map_err(|_| "Overlay registry is unavailable".to_string())?;
    if known.len() == monitors.len()
        && known.iter().all(|previous| {
            monitors
                .iter()
                .find(|current| current.label == previous.label)
                .is_some_and(|current| same_layout(previous, current))
        })
    {
        *known = monitors.to_vec();
        return Ok(started.elapsed().as_secs_f64() * 1000.0);
    }

    let desired: Vec<_> = monitors
        .iter()
        .map(|monitor| monitor.label.as_str())
        .collect();
    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-") && !desired.contains(&label.as_str()) {
            window
                .close()
                .map_err(|error| format!("Could not close stale overlay {label}: {error}"))?;
        }
    }

    for monitor in monitors {
        let window = match app.get_webview_window(&monitor.label) {
            Some(window) => window,
            None => create_overlay(app, monitor)?,
        };
        position_window(&window, monitor)?;
    }
    *known = monitors.to_vec();
    Ok(started.elapsed().as_secs_f64() * 1000.0)
}

fn same_layout(left: &MonitorInfo, right: &MonitorInfo) -> bool {
    left.x == right.x
        && left.y == right.y
        && left.width == right.width
        && left.height == right.height
        && left.scale_factor == right.scale_factor
}

fn create_overlay(app: &AppHandle, monitor: &MonitorInfo) -> Result<WebviewWindow, String> {
    WebviewWindowBuilder::new(app, &monitor.label, WebviewUrl::App("overlay".into()))
        .title("KeepShot Capture")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|error| format!("Could not create {}: {error}", monitor.label))
}

fn position_window(window: &WebviewWindow, monitor: &MonitorInfo) -> Result<(), String> {
    let position = PhysicalPosition::new(monitor.x, monitor.y);
    let size = PhysicalSize::new(monitor.width, monitor.height);
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
        eprintln!(
            "Overlay geometry mismatch for {}: wanted ({}, {}) {}x{}, got ({}, {}) {}x{}",
            monitor.label,
            monitor.x,
            monitor.y,
            monitor.width,
            monitor.height,
            actual_position.x,
            actual_position.y,
            actual_size.width,
            actual_size.height,
        );
    }
    Ok(())
}
