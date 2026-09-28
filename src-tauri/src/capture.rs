use std::thread;
use std::time::Instant;

use image::RgbaImage;
use serde::Serialize;
use xcap::Monitor;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub label: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub struct CapturedFrame {
    pub monitor: MonitorInfo,
    pub bytes: Vec<u8>,
    pub capture_ms: f64,
    pub prepare_ms: f64,
}

pub fn monitor_infos() -> Result<Vec<MonitorInfo>, String> {
    Monitor::all()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|monitor| {
            let id = monitor.id().map_err(|error| error.to_string())?;
            let name = monitor
                .friendly_name()
                .or_else(|_| monitor.name())
                .map_err(|error| error.to_string())?;
            Ok(MonitorInfo {
                label: format!("monitor-{id}"),
                name,
                x: monitor.x().map_err(|error| error.to_string())?,
                y: monitor.y().map_err(|error| error.to_string())?,
                width: monitor.width().map_err(|error| error.to_string())?,
                height: monitor.height().map_err(|error| error.to_string())?,
                scale_factor: monitor.scale_factor().map_err(|error| error.to_string())?,
            })
        })
        .collect()
}

pub fn virtual_bounds(monitors: &[MonitorInfo]) -> Result<VirtualBounds, String> {
    let Some(first) = monitors.first() else {
        return Err("No monitors were found".to_string());
    };
    let mut min_x = i64::from(first.x);
    let mut min_y = i64::from(first.y);
    let mut max_x = min_x + i64::from(first.width);
    let mut max_y = min_y + i64::from(first.height);
    for monitor in &monitors[1..] {
        let x = i64::from(monitor.x);
        let y = i64::from(monitor.y);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + i64::from(monitor.width));
        max_y = max_y.max(y + i64::from(monitor.height));
    }
    let width =
        u32::try_from(max_x - min_x).map_err(|_| "Virtual desktop is too wide".to_string())?;
    let height =
        u32::try_from(max_y - min_y).map_err(|_| "Virtual desktop is too tall".to_string())?;
    Ok(VirtualBounds {
        x: i32::try_from(min_x)
            .map_err(|_| "Virtual desktop x coordinate is out of range".to_string())?,
        y: i32::try_from(min_y)
            .map_err(|_| "Virtual desktop y coordinate is out of range".to_string())?,
        width,
        height,
    })
}

pub fn capture_all(monitors: &[MonitorInfo]) -> Result<Vec<CapturedFrame>, String> {
    if monitors.is_empty() {
        return Err("No monitors were found".to_string());
    }
    thread::scope(|scope| {
        let handles: Vec<_> = monitors
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, info)| scope.spawn(move || capture_index(index, info)))
            .collect();
        let mut frames = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.join() {
                Ok(Ok(frame)) => frames.push(frame),
                Ok(Err(error)) => return Err(error),
                Err(_) => return Err("A monitor capture worker panicked".to_string()),
            }
        }
        Ok(frames)
    })
}

fn capture_index(index: usize, info: MonitorInfo) -> Result<CapturedFrame, String> {
    let monitor = Monitor::all()
        .map_err(|error| error.to_string())?
        .into_iter()
        .nth(index)
        .ok_or_else(|| format!("Monitor {} disappeared during capture", info.label))?;
    let capture_started = Instant::now();
    let image = monitor
        .capture_image()
        .map_err(|error| format!("Capture failed for {}: {error}", info.label))?;
    let capture_ms = capture_started.elapsed().as_secs_f64() * 1000.0;
    if image.width() != info.width || image.height() != info.height {
        return Err(format!(
            "Monitor geometry changed during capture for {}",
            info.label
        ));
    }
    let prepare_started = Instant::now();
    let bytes = into_opaque_rgba(image);
    let prepare_ms = prepare_started.elapsed().as_secs_f64() * 1000.0;
    Ok(CapturedFrame {
        monitor: info,
        bytes,
        capture_ms,
        prepare_ms,
    })
}

/// GDI captures can carry a zero alpha channel; the overlay canvas and exports need opaque pixels.
fn into_opaque_rgba(image: RgbaImage) -> Vec<u8> {
    let mut bytes = image.into_raw();
    for pixel in bytes.as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    bytes
}
