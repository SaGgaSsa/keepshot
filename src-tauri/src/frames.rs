use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::capture::{CapturedFrame, MonitorInfo, VirtualBounds};
use crate::output::SelectionRect;

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn next_session_id() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingFrames {
    pub session: String,
    pub bounds: VirtualBounds,
    pub monitors: Vec<MonitorInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit: Option<EditFrameInfo>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditFrameInfo {
    pub history_id: String,
    pub rect: SelectionRect,
    pub document: Value,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendImageTiming {
    pub label: String,
    pub load_ms: f64,
    pub paint_ms: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorMetrics {
    pub label: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub capture_ms: f64,
    pub prepare_ms: f64,
    pub load_ms: f64,
    pub paint_ms: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureMetrics {
    pub session: String,
    pub capture_total_ms: f64,
    pub emit_ms: f64,
    pub shown_ms: f64,
    pub monitors: Vec<MonitorMetrics>,
}

#[derive(Clone)]
pub struct FrameEntry {
    pub monitor: MonitorInfo,
    pub bytes: Arc<Vec<u8>>,
    pub offset_x: u32,
    pub offset_y: u32,
    pub capture_ms: f64,
    pub prepare_ms: f64,
}

impl FrameEntry {
    pub fn new(frame: CapturedFrame) -> Self {
        Self {
            monitor: frame.monitor,
            bytes: Arc::new(frame.bytes),
            offset_x: 0,
            offset_y: 0,
            capture_ms: frame.capture_ms,
            prepare_ms: frame.prepare_ms,
        }
    }
}

struct FrameSession {
    id: String,
    started: Instant,
    capture_total_ms: f64,
    emit_ms: Option<f64>,
    metrics_emitted: bool,
    bounds: VirtualBounds,
    entries: HashMap<String, FrameEntry>,
    edit: Option<EditFrameInfo>,
}

#[derive(Default)]
pub struct FrameStore {
    inner: Mutex<Option<FrameSession>>,
    busy: AtomicBool,
}

impl FrameStore {
    pub fn try_start(&self) -> bool {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn begin(
        &self,
        id: String,
        started: Instant,
        capture_total_ms: f64,
        bounds: VirtualBounds,
        frames: Vec<FrameEntry>,
    ) -> Result<(), String> {
        self.begin_with_edit(id, started, capture_total_ms, bounds, frames, None)
    }

    pub fn begin_edit(
        &self,
        id: String,
        started: Instant,
        bounds: VirtualBounds,
        frames: Vec<FrameEntry>,
        edit: EditFrameInfo,
    ) -> Result<(), String> {
        self.begin_with_edit(id, started, 0.0, bounds, frames, Some(edit))
    }

    fn begin_with_edit(
        &self,
        id: String,
        started: Instant,
        capture_total_ms: f64,
        bounds: VirtualBounds,
        frames: Vec<FrameEntry>,
        edit: Option<EditFrameInfo>,
    ) -> Result<(), String> {
        let mut current = self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())?;
        let mut entries = HashMap::with_capacity(frames.len());
        for mut frame in frames {
            frame.offset_x = u32::try_from(i64::from(frame.monitor.x) - i64::from(bounds.x))
                .map_err(|_| "Monitor x offset is invalid".to_string())?;
            frame.offset_y = u32::try_from(i64::from(frame.monitor.y) - i64::from(bounds.y))
                .map_err(|_| "Monitor y offset is invalid".to_string())?;
            entries.insert(frame.monitor.label.clone(), frame);
        }
        *current = Some(FrameSession {
            id,
            started,
            capture_total_ms,
            emit_ms: None,
            metrics_emitted: false,
            bounds,
            entries,
            edit,
        });
        Ok(())
    }

    pub fn set_emit_time(&self) -> Result<(), String> {
        let mut current = self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())?;
        if let Some(session) = current.as_mut() {
            session.emit_ms = Some(session.started.elapsed().as_secs_f64() * 1000.0);
        }
        Ok(())
    }

    pub fn elapsed_ms(&self) -> Result<f64, String> {
        let current = self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())?;
        current
            .as_ref()
            .map(|session| session.started.elapsed().as_secs_f64() * 1000.0)
            .ok_or_else(|| "Capture session is no longer active".to_string())
    }

    pub fn pending(&self) -> Option<PendingFrames> {
        let current = self.inner.lock().ok()?;
        let session = current.as_ref()?;
        let mut monitors: Vec<_> = session
            .entries
            .values()
            .map(|frame| frame.monitor.clone())
            .collect();
        monitors.sort_by(|left, right| left.label.cmp(&right.label));
        Some(PendingFrames {
            session: session.id.clone(),
            bounds: session.bounds,
            monitors,
            edit: session.edit.clone(),
        })
    }

    /// Returns rows `start_row..end_row` of a frame. The overlay fetches frames as
    /// parallel row bands because WebView2 custom-protocol throughput is per request.
    pub fn frame_rows(
        &self,
        label: &str,
        session_id: &str,
        start_row: u32,
        end_row: u32,
    ) -> Option<Vec<u8>> {
        let (bytes, width, height) = {
            let current = self.inner.lock().ok()?;
            let session = current
                .as_ref()
                .filter(|session| session.id == session_id)?;
            let frame = session.entries.get(label)?;
            (
                Arc::clone(&frame.bytes),
                frame.monitor.width,
                frame.monitor.height,
            )
        };
        if start_row >= end_row || end_row > height {
            return None;
        }
        let stride = width as usize * 4;
        bytes
            .get(start_row as usize * stride..end_row as usize * stride)
            .map(<[u8]>::to_vec)
    }

    pub fn validate_ready(
        &self,
        session_id: &str,
        timings: &[FrontendImageTiming],
    ) -> Result<(), String> {
        let current = self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())?;
        let session = current
            .as_ref()
            .filter(|session| session.id == session_id)
            .ok_or_else(|| "Capture session is no longer active".to_string())?;
        let expected: HashSet<_> = session.entries.keys().map(String::as_str).collect();
        let received: HashSet<_> = timings.iter().map(|timing| timing.label.as_str()).collect();
        if expected != received || timings.len() != expected.len() {
            return Err("The overlay did not decode every monitor frame".to_string());
        }
        if timings
            .iter()
            .any(|timing| !timing.load_ms.is_finite() || !timing.paint_ms.is_finite())
        {
            return Err("Overlay image timings are invalid".to_string());
        }
        Ok(())
    }

    pub fn mark_ready(
        &self,
        session_id: &str,
        timings: Vec<FrontendImageTiming>,
        shown_ms: f64,
    ) -> Result<Option<CaptureMetrics>, String> {
        let mut current = self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())?;
        let Some(session) = current.as_mut().filter(|session| session.id == session_id) else {
            return Ok(None);
        };
        if session.metrics_emitted {
            return Ok(None);
        }
        if session.edit.is_some() {
            session.metrics_emitted = true;
            return Ok(None);
        }
        let timing_map: HashMap<_, _> = timings
            .into_iter()
            .map(|timing| (timing.label.clone(), timing))
            .collect();
        let mut monitors = Vec::with_capacity(session.entries.len());
        for (label, frame) in &session.entries {
            let Some(timing) = timing_map.get(label) else {
                return Err("A monitor timing was not provided".to_string());
            };
            monitors.push(MonitorMetrics {
                label: label.clone(),
                name: frame.monitor.name.clone(),
                x: frame.monitor.x,
                y: frame.monitor.y,
                width: frame.monitor.width,
                height: frame.monitor.height,
                scale_factor: frame.monitor.scale_factor,
                capture_ms: frame.capture_ms,
                prepare_ms: frame.prepare_ms,
                load_ms: timing.load_ms,
                paint_ms: timing.paint_ms,
            });
        }
        monitors.sort_by(|left, right| left.label.cmp(&right.label));
        session.metrics_emitted = true;
        Ok(Some(CaptureMetrics {
            session: session.id.clone(),
            capture_total_ms: session.capture_total_ms,
            emit_ms: session.emit_ms.unwrap_or_default(),
            shown_ms,
            monitors,
        }))
    }

    pub fn snapshot(&self) -> Result<(String, VirtualBounds, Vec<FrameEntry>), String> {
        let current = self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())?;
        let session = current
            .as_ref()
            .ok_or_else(|| "There is no active capture to export".to_string())?;
        Ok((
            session.id.clone(),
            session.bounds,
            session.entries.values().cloned().collect(),
        ))
    }

    pub fn validate_history_target(&self, history_id: Option<&str>) -> Result<(), String> {
        let current = self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())?;
        let session = current
            .as_ref()
            .ok_or_else(|| "There is no active capture to export".to_string())?;
        let expected = session.edit.as_ref().map(|edit| edit.history_id.as_str());
        if expected != history_id {
            return Err("Export history id does not match the active session".to_string());
        }
        Ok(())
    }

    pub fn clear_if_session(&self, session_id: &str) -> Result<bool, String> {
        let mut current = self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())?;
        if !matches!(current.as_ref(), Some(session) if session.id == session_id) {
            return Ok(false);
        }
        *current = None;
        self.busy.store(false, Ordering::Release);
        Ok(true)
    }

    pub fn clear(&self) -> Result<(), String> {
        *self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())? = None;
        self.busy.store(false, Ordering::Release);
        Ok(())
    }
}
