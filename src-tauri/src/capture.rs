use std::thread;
use std::time::Instant;

use image::codecs::bmp::BmpEncoder;
use image::{ExtendedColorType, ImageEncoder};
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

struct CaptureTarget {
    monitor: Monitor,
    info: MonitorInfo,
}

pub struct CapturedFrame {
    pub monitor: MonitorInfo,
    pub bytes: Vec<u8>,
    pub capture_ms: f64,
    pub encode_ms: f64,
}

fn targets() -> Result<Vec<CaptureTarget>, String> {
    Monitor::all()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|monitor| {
            let id = monitor.id().map_err(|error| error.to_string())?;
            let name = monitor
                .friendly_name()
                .or_else(|_| monitor.name())
                .map_err(|error| error.to_string())?;
            Ok(CaptureTarget {
                info: MonitorInfo {
                    label: format!("overlay-{id}"),
                    name,
                    x: monitor.x().map_err(|error| error.to_string())?,
                    y: monitor.y().map_err(|error| error.to_string())?,
                    width: monitor.width().map_err(|error| error.to_string())?,
                    height: monitor.height().map_err(|error| error.to_string())?,
                    scale_factor: monitor.scale_factor().map_err(|error| error.to_string())?,
                },
                monitor,
            })
        })
        .collect()
}

pub fn monitor_infos() -> Result<Vec<MonitorInfo>, String> {
    let mut monitors: Vec<_> = targets()?.into_iter().map(|target| target.info).collect();
    monitors.sort_by(|left, right| left.label.cmp(&right.label));
    Ok(monitors)
}

pub fn capture_all() -> Result<Vec<CapturedFrame>, String> {
    let monitor_count = Monitor::all().map_err(|error| error.to_string())?.len();
    if monitor_count == 0 {
        return Err("No monitors were found".to_string());
    }
    thread::scope(|scope| {
        let handles: Vec<_> = (0..monitor_count)
            .map(|index| scope.spawn(move || capture_index(index)))
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

fn capture_index(index: usize) -> Result<CapturedFrame, String> {
    let monitor = Monitor::all()
        .map_err(|error| error.to_string())?
        .into_iter()
        .nth(index)
        .ok_or_else(|| format!("Monitor at index {index} disappeared during capture"))?;
    let id = monitor.id().map_err(|error| error.to_string())?;
    let name = monitor
        .friendly_name()
        .or_else(|_| monitor.name())
        .map_err(|error| error.to_string())?;
    let info = MonitorInfo {
        label: format!("overlay-{id}"),
        name,
        x: monitor.x().map_err(|error| error.to_string())?,
        y: monitor.y().map_err(|error| error.to_string())?,
        width: monitor.width().map_err(|error| error.to_string())?,
        height: monitor.height().map_err(|error| error.to_string())?,
        scale_factor: monitor.scale_factor().map_err(|error| error.to_string())?,
    };
    capture_one(CaptureTarget { monitor, info })
}

fn capture_one(target: CaptureTarget) -> Result<CapturedFrame, String> {
    let capture_started = Instant::now();
    let image = target
        .monitor
        .capture_image()
        .map_err(|error| format!("Capture failed for {}: {error}", target.info.label))?;
    let capture_ms = capture_started.elapsed().as_secs_f64() * 1000.0;
    let encode_started = Instant::now();
    let mut bytes =
        Vec::with_capacity((image.width() as usize * image.height() as usize * 4) + 138);
    BmpEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|error| format!("BMP encoding failed for {}: {error}", target.info.label))?;
    let encode_ms = encode_started.elapsed().as_secs_f64() * 1000.0;
    Ok(CapturedFrame {
        monitor: target.info,
        bytes,
        capture_ms,
        encode_ms,
    })
}
