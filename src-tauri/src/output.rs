use std::path::{Path, PathBuf};

use image::{ColorType, Rgba, RgbaImage};
use serde::Deserialize;

use crate::capture::VirtualBounds;
use crate::frames::{FrameEntry, BMP_PIXEL_OFFSET};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn compose(
    bounds: VirtualBounds,
    frames: Vec<FrameEntry>,
    rect: SelectionRect,
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
    if rect_left < bounds_left
        || rect_top < bounds_top
        || rect_right > bounds_right
        || rect_bottom > bounds_bottom
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
    // GDI captures can carry a zero alpha channel; exported images must be opaque.
    for pixel in output.pixels_mut() {
        pixel.0[3] = 255;
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
        let source_offset = BMP_PIXEL_OFFSET
            .checked_add(
                (source_y + row)
                    .checked_mul(source_stride)
                    .ok_or_else(|| "Source row offset is too large".to_string())?,
            )
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
            .ok_or_else(|| format!("BMP pixel data is truncated for {}", frame.monitor.label))?;
        let destination_row = output_bytes
            .get_mut(destination_offset..destination_end)
            .ok_or_else(|| "Selection buffer is smaller than expected".to_string())?;
        destination_row.copy_from_slice(source_row);
    }
    Ok(())
}

pub fn save_png(path: &Path, image: &RgbaImage) -> Result<(), String> {
    image::save_buffer(
        path,
        image.as_raw(),
        image.width(),
        image.height(),
        ColorType::Rgba8,
    )
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
