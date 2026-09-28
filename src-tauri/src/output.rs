use std::path::{Path, PathBuf};

use image::{ColorType, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::capture::VirtualBounds;
use crate::frames::FrameEntry;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportHeader {
    rect: SelectionRect,
    document: serde_json::Value,
    has_composite: bool,
    has_redact_base: bool,
    history_id: Option<String>,
}

pub struct ExportPayload {
    pub rect: SelectionRect,
    pub document: serde_json::Value,
    pub composite: Option<Vec<u8>>,
    pub redact_base: Option<Vec<u8>>,
    pub history_id: Option<String>,
}

pub fn parse_export_body(bytes: &[u8]) -> Result<ExportPayload, String> {
    if bytes.len() < 4 {
        return Err("Export body is missing its JSON length prefix".to_string());
    }
    let json_length = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let json_end = 4usize
        .checked_add(json_length)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| "Export body has an invalid JSON length".to_string())?;
    let header: ExportHeader = serde_json::from_slice(&bytes[4..json_end])
        .map_err(|error| format!("Invalid export metadata: {error}"))?;
    if !header.document.is_object()
        || header
            .document
            .get("version")
            .and_then(serde_json::Value::as_u64)
            != Some(2)
        || !header
            .document
            .get("layers")
            .is_some_and(serde_json::Value::is_array)
    {
        return Err("Export annotation document must be version 2".to_string());
    }
    if header.rect.width == 0 || header.rect.height == 0 {
        return Err("Selection width and height must be at least one pixel".to_string());
    }
    let layer_bytes = (header.rect.width as usize)
        .checked_mul(header.rect.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Selection is too large".to_string())?;
    let has_redact_layers = header.document["layers"]
        .as_array()
        .is_some_and(|layers| layers.iter().any(|layer| layer["type"] == "redact"));
    if has_redact_layers {
        return Err("Redaction layers must not be included in export metadata".to_string());
    }
    let document_has_layers = header.document["layers"]
        .as_array()
        .is_some_and(|layers| !layers.is_empty());
    if !header.has_composite && (document_has_layers || header.has_redact_base) {
        return Err("Export metadata is missing its composite image".to_string());
    }
    let composite_length = if header.has_composite { layer_bytes } else { 0 };
    let redact_length = if header.has_redact_base {
        layer_bytes
    } else {
        0
    };
    let expected_length = json_end
        .checked_add(composite_length)
        .and_then(|length| length.checked_add(redact_length))
        .ok_or_else(|| "Export body is too large".to_string())?;
    if bytes.len() != expected_length {
        return Err("Export body image lengths do not match the selection".to_string());
    }
    let composite_end = json_end + composite_length;
    Ok(ExportPayload {
        rect: header.rect,
        document: header.document,
        composite: header
            .has_composite
            .then(|| bytes[json_end..composite_end].to_vec()),
        redact_base: header
            .has_redact_base
            .then(|| bytes[composite_end..].to_vec()),
        history_id: header.history_id,
    })
}

pub fn compose(
    bounds: VirtualBounds,
    frames: Vec<FrameEntry>,
    rect: SelectionRect,
    overlay: Option<&[u8]>,
) -> Result<RgbaImage, String> {
    compose_region(bounds, frames, rect, overlay, false)
}

pub fn compose_history_region(
    bounds: VirtualBounds,
    frames: Vec<FrameEntry>,
    rect: SelectionRect,
    overlay: Option<&[u8]>,
) -> Result<RgbaImage, String> {
    compose_region(bounds, frames, rect, overlay, true)
}

fn compose_region(
    bounds: VirtualBounds,
    frames: Vec<FrameEntry>,
    rect: SelectionRect,
    overlay: Option<&[u8]>,
    allow_outside_bounds: bool,
) -> Result<RgbaImage, String> {
    if rect.width == 0 || rect.height == 0 {
        return Err("Selection width and height must be at least one pixel".to_string());
    }
    let rect_left = i64::from(rect.x);
    let rect_top = i64::from(rect.y);
    let rect_right = rect_left + i64::from(rect.width);
    let rect_bottom = rect_top + i64::from(rect.height);
    let bounds_left = i64::from(bounds.x);
    let bounds_top = i64::from(bounds.y);
    let bounds_right = bounds_left + i64::from(bounds.width);
    let bounds_bottom = bounds_top + i64::from(bounds.height);
    if !allow_outside_bounds
        && (rect_left < bounds_left
            || rect_top < bounds_top
            || rect_right > bounds_right
            || rect_bottom > bounds_bottom)
    {
        return Err("Selection is outside the virtual desktop bounds".to_string());
    }

    let mut output = RgbaImage::from_pixel(rect.width, rect.height, Rgba([0, 0, 0, 255]));
    for frame in frames {
        let monitor_left = bounds_left + i64::from(frame.offset_x);
        let monitor_top = bounds_top + i64::from(frame.offset_y);
        let monitor_right = monitor_left + i64::from(frame.monitor.width);
        let monitor_bottom = monitor_top + i64::from(frame.monitor.height);
        let left = rect_left.max(monitor_left);
        let top = rect_top.max(monitor_top);
        let right = rect_right.min(monitor_right);
        let bottom = rect_bottom.min(monitor_bottom);
        if left >= right || top >= bottom {
            continue;
        }
        copy_intersection(
            &mut output,
            &frame,
            CopyArea {
                rect_left,
                rect_top,
                monitor_left,
                monitor_top,
                left,
                top,
                right,
                bottom,
            },
        )?;
    }
    if let Some(overlay) = overlay {
        let expected = (rect.width as usize)
            .checked_mul(rect.height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "Annotation layer is too large".to_string())?;
        if overlay.len() != expected {
            return Err("Annotation layer size does not match the selection".to_string());
        }
        let (destination, remainder) = output.as_mut().as_chunks_mut::<4>();
        if !remainder.is_empty() {
            return Err("Selection buffer has an invalid RGBA size".to_string());
        }
        for (pixel, source) in destination.iter_mut().zip(overlay.as_chunks::<4>().0) {
            let alpha = u32::from(source[3]);
            let inverse = 255 - alpha;
            for (destination, source) in pixel.iter_mut().zip(source.iter()).take(3) {
                *destination =
                    ((u32::from(*source) * alpha + u32::from(*destination) * inverse + 127) / 255)
                        as u8;
            }
            pixel[3] = 255;
        }
    }
    Ok(output)
}

struct CopyArea {
    rect_left: i64,
    rect_top: i64,
    monitor_left: i64,
    monitor_top: i64,
    left: i64,
    top: i64,
    right: i64,
    bottom: i64,
}

fn copy_intersection(
    output: &mut RgbaImage,
    frame: &FrameEntry,
    area: CopyArea,
) -> Result<(), String> {
    let source_x = usize::try_from(area.left - area.monitor_left)
        .map_err(|_| "Invalid source x coordinate".to_string())?;
    let source_y = usize::try_from(area.top - area.monitor_top)
        .map_err(|_| "Invalid source y coordinate".to_string())?;
    let destination_x = usize::try_from(area.left - area.rect_left)
        .map_err(|_| "Invalid destination x coordinate".to_string())?;
    let destination_y = usize::try_from(area.top - area.rect_top)
        .map_err(|_| "Invalid destination y coordinate".to_string())?;
    let copy_width =
        usize::try_from(area.right - area.left).map_err(|_| "Invalid copy width".to_string())?;
    let copy_height =
        usize::try_from(area.bottom - area.top).map_err(|_| "Invalid copy height".to_string())?;
    let monitor_width = frame.monitor.width as usize;
    let source = frame.bytes.as_slice();
    let source_stride = monitor_width
        .checked_mul(4)
        .ok_or_else(|| "Monitor stride is too large".to_string())?;
    let copy_bytes = copy_width
        .checked_mul(4)
        .ok_or_else(|| "Copy row is too large".to_string())?;
    let output_stride = output.width() as usize * 4;
    let output_bytes = output.as_mut();

    for row in 0..copy_height {
        let source_offset = (source_y + row)
            .checked_mul(source_stride)
            .and_then(|offset| offset.checked_add(source_x.checked_mul(4)?))
            .ok_or_else(|| "Source pixel offset is too large".to_string())?;
        let source_end = source_offset
            .checked_add(copy_bytes)
            .ok_or_else(|| "Source row end is too large".to_string())?;
        let destination_offset = (destination_y + row)
            .checked_mul(output_stride)
            .and_then(|offset| offset.checked_add(destination_x.checked_mul(4)?))
            .ok_or_else(|| "Destination row offset is too large".to_string())?;
        let destination_end = destination_offset
            .checked_add(copy_bytes)
            .ok_or_else(|| "Destination row end is too large".to_string())?;
        let source_row = source
            .get(source_offset..source_end)
            .ok_or_else(|| format!("Frame pixel data is truncated for {}", frame.monitor.label))?;
        let destination_row = output_bytes
            .get_mut(destination_offset..destination_end)
            .ok_or_else(|| "Selection buffer is smaller than expected".to_string())?;
        destination_row.copy_from_slice(source_row);
    }
    Ok(())
}

/// Captures are always opaque, so drop the alpha channel for smaller, more compatible PNGs.
pub fn save_png(path: &Path, image: &RgbaImage) -> Result<(), String> {
    let rgb: Vec<u8> = image
        .as_raw()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[red, green, blue, _]| [red, green, blue])
        .collect();
    image::save_buffer(path, &rgb, image.width(), image.height(), ColorType::Rgb8)
        .map_err(|error| format!("Could not save {}: {error}", path.display()))
}

pub fn unique_capture_path(directory: &Path, filename: &str) -> PathBuf {
    let initial = directory.join(filename);
    if !initial.exists() {
        return initial;
    }
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("KeepShot_capture");
    let extension = Path::new(filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("png");
    let mut suffix = 2u32;
    loop {
        let candidate = directory.join(format!("{stem}_{suffix}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
        suffix = suffix.saturating_add(1);
    }
}
