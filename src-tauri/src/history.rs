use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local, Timelike};
use image::{imageops::FilterType, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::output::{save_png, SelectionRect};

const HISTORY_LIMIT: usize = 30;
static HISTORY_WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMeta {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    #[serde(flatten)]
    pub meta: HistoryMeta,
    pub thumb_url: String,
}

pub struct EditableCapture {
    pub meta: HistoryMeta,
    pub original: RgbaImage,
    pub document: Value,
}

pub fn prepare(root: &Path) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|error| {
        format!(
            "Could not create history folder {}: {error}",
            root.display()
        )
    })?;
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let path = entry.path();
        if name.ends_with(".tmp") && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            fs::remove_dir_all(entry.path()).map_err(|error| {
                format!(
                    "Could not remove incomplete history entry {}: {error}",
                    entry.path().display()
                )
            })?;
        } else if let Some(id) = name.strip_suffix(".old") {
            if !entry.file_type().is_ok_and(|kind| kind.is_dir()) || !valid_id(id) {
                continue;
            }
            let destination = root.join(id);
            if destination.exists() {
                fs::remove_dir_all(path).map_err(|error| error.to_string())?;
            } else {
                fs::rename(path, destination).map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(())
}

pub fn list(root: &Path) -> Result<Vec<HistoryItem>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        if !valid_id(&id) {
            continue;
        }
        let directory = entry.path();
        let meta_path = directory.join("meta.json");
        let meta = match fs::read(&meta_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<HistoryMeta>(&bytes).ok())
        {
            Some(meta) if meta.id == id && directory.join("thumb.png").is_file() => meta,
            _ => continue,
        };
        items.push(HistoryItem {
            thumb_url: format!("http://history.localhost/{id}/thumb.png"),
            meta,
        });
    }
    items.sort_by(|left, right| {
        let left_time = DateTime::parse_from_rfc3339(&left.meta.updated_at).ok();
        let right_time = DateTime::parse_from_rfc3339(&right.meta.updated_at).ok();
        right_time
            .cmp(&left_time)
            .then_with(|| right.meta.updated_at.cmp(&left.meta.updated_at))
    });
    Ok(items)
}

pub fn record(
    root: &Path,
    rect: SelectionRect,
    base: &RgbaImage,
    final_image: &RgbaImage,
    redact_base: Option<&RgbaImage>,
    document: &Value,
    history_id: Option<&str>,
) -> Result<String, String> {
    let _guard = HISTORY_WRITE_LOCK
        .lock()
        .map_err(|_| "History storage is unavailable".to_string())?;
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let (id, created_at, original) = if let Some(id) = history_id {
        validate_id(id)?;
        let destination = root.join(id);
        let meta = read_meta(&destination)?;
        let original = match redact_base {
            Some(image) => image.clone(),
            None => image::open(destination.join("original.png"))
                .map_err(|error| format!("Could not load original history image: {error}"))?
                .to_rgba8(),
        };
        (id.to_string(), meta.created_at, original)
    } else {
        let id = new_id(root)?;
        let original = redact_base.cloned().unwrap_or_else(|| base.clone());
        (id, Local::now().to_rfc3339(), original)
    };
    let updated_at = Local::now().to_rfc3339();
    let meta = HistoryMeta {
        id: id.clone(),
        created_at,
        updated_at,
        width: rect.width,
        height: rect.height,
    };
    let directory = root.join(&id);
    let stage = root.join(format!("{id}.tmp"));
    if stage.exists() {
        fs::remove_dir_all(&stage).map_err(|error| error.to_string())?;
    }
    fs::create_dir(&stage).map_err(|error| error.to_string())?;
    let result = write_entry(&stage, &meta, &original, final_image, document);
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }
    replace_directory(root, &id, &stage, &directory, history_id.is_some())?;
    prune(root)?;
    Ok(id)
}

fn write_entry(
    directory: &Path,
    meta: &HistoryMeta,
    original: &RgbaImage,
    final_image: &RgbaImage,
    document: &Value,
) -> Result<(), String> {
    save_png(&directory.join("original.png"), original)?;
    save_png(&directory.join("final.png"), final_image)?;
    let longest = final_image.width().max(final_image.height()).max(1);
    let width = ((u64::from(final_image.width()) * 320) / u64::from(longest)).max(1) as u32;
    let height = ((u64::from(final_image.height()) * 320) / u64::from(longest)).max(1) as u32;
    let thumbnail = image::imageops::resize(final_image, width, height, FilterType::Triangle);
    save_png(&directory.join("thumb.png"), &thumbnail)?;
    let meta_bytes = serde_json::to_vec_pretty(meta).map_err(|error| error.to_string())?;
    fs::write(directory.join("meta.json"), meta_bytes).map_err(|error| error.to_string())?;
    let document_bytes = serde_json::to_vec_pretty(document).map_err(|error| error.to_string())?;
    fs::write(directory.join("annotations.json"), document_bytes)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn replace_directory(
    root: &Path,
    id: &str,
    stage: &Path,
    destination: &Path,
    update: bool,
) -> Result<(), String> {
    if !update {
        return fs::rename(stage, destination)
            .map_err(|error| format!("Could not commit history entry {id}: {error}"));
    }
    let backup = root.join(format!("{id}.old"));
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|error| error.to_string())?;
    }
    fs::rename(destination, &backup)
        .map_err(|error| format!("Could not stage history update {id}: {error}"))?;
    if let Err(error) = fs::rename(stage, destination) {
        let rollback = fs::rename(&backup, destination);
        return Err(format!(
            "Could not commit history update {id}: {error}{}",
            rollback
                .err()
                .map(|error| format!("; rollback failed: {error}"))
                .unwrap_or_default()
        ));
    }
    fs::remove_dir_all(backup).map_err(|error| error.to_string())
}

fn prune(root: &Path) -> Result<(), String> {
    let items = list(root)?;
    for item in items.into_iter().skip(HISTORY_LIMIT) {
        validate_id(&item.meta.id)?;
        fs::remove_dir_all(root.join(item.meta.id)).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn load_edit(root: &Path, id: &str) -> Result<EditableCapture, String> {
    validate_id(id)?;
    let directory = root.join(id);
    let meta = read_meta(&directory)?;
    let original = image::open(directory.join("original.png"))
        .map_err(|error| format!("Could not read original history image: {error}"))?
        .to_rgba8();
    let document_bytes = fs::read(directory.join("annotations.json"))
        .map_err(|error| format!("Could not read annotation document: {error}"))?;
    let document = serde_json::from_slice(&document_bytes)
        .map_err(|error| format!("Could not read annotation document: {error}"))?;
    Ok(EditableCapture {
        meta,
        original,
        document,
    })
}

pub fn load_final(root: &Path, id: &str) -> Result<RgbaImage, String> {
    validate_id(id)?;
    image::open(root.join(id).join("final.png"))
        .map(image::DynamicImage::into_rgba8)
        .map_err(|error| format!("Could not read final history image: {error}"))
}

pub fn thumbnail_path(root: &Path, id: &str) -> Option<PathBuf> {
    if !valid_id(id) {
        return None;
    }
    let directory = root.join(id);
    if !directory.is_dir() || !directory.join("meta.json").is_file() {
        return None;
    }
    let meta = read_meta(&directory).ok()?;
    (meta.id == id).then(|| directory.join("thumb.png"))
}

pub fn delete(root: &Path, id: &str) -> Result<(), String> {
    validate_id(id)?;
    let directory = root.join(id);
    if !directory.exists() {
        return Err("History entry does not exist".to_string());
    }
    fs::remove_dir_all(directory).map_err(|error| error.to_string())
}

fn read_meta(directory: &Path) -> Result<HistoryMeta, String> {
    let bytes = fs::read(directory.join("meta.json")).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn new_id(root: &Path) -> Result<String, String> {
    let now = Local::now();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.subsec_nanos());
    let mixed = nanos ^ now.nanosecond();
    for offset in 0..=u16::MAX {
        let suffix = (mixed as u16).wrapping_add(offset);
        let id = format!("{}-{suffix:04x}", now.format("%Y%m%d-%H%M%S"));
        if !root.join(&id).exists() && !root.join(format!("{id}.tmp")).exists() {
            return Ok(id);
        }
    }
    Err("Could not allocate a unique history id".to_string())
}

fn validate_id(id: &str) -> Result<(), String> {
    if valid_id(id) {
        Ok(())
    } else {
        Err("Invalid history entry id".to_string())
    }
}

fn valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 20
        && bytes[8] == b'-'
        && bytes[15] == b'-'
        && bytes[..8].iter().all(u8::is_ascii_digit)
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[16..].iter().all(u8::is_ascii_hexdigit)
}
