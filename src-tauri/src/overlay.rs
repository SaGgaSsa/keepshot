use std::sync::Mutex;
use std::time::Instant;

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
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
        eprintln!(
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
