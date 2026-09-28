use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;

use crate::capture::{CapturedFrame, MonitorInfo};

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
pub struct PendingFrame {
    pub session: String,
    pub monitor: MonitorInfo,
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
    pub encode_ms: f64,
    pub bytes: usize,
    pub shown_ms: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureMetrics {
    pub session: String,
    pub reconciliation_ms: f64,
    pub capture_total_ms: f64,
    pub monitors: Vec<MonitorMetrics>,
}

#[derive(Clone)]
pub struct FrameEntry {
    pub monitor: MonitorInfo,
    pub bytes: Arc<Vec<u8>>,
    pub capture_ms: f64,
    pub encode_ms: f64,
    pub shown_ms: Option<f64>,
}

impl FrameEntry {
    pub fn new(frame: CapturedFrame) -> Self {
        Self {
            monitor: frame.monitor,
            bytes: Arc::new(frame.bytes),
            capture_ms: frame.capture_ms,
            encode_ms: frame.encode_ms,
            shown_ms: None,
        }
    }

    fn metrics(&self) -> MonitorMetrics {
        MonitorMetrics {
            label: self.monitor.label.clone(),
            name: self.monitor.name.clone(),
            x: self.monitor.x,
            y: self.monitor.y,
            width: self.monitor.width,
            height: self.monitor.height,
            scale_factor: self.monitor.scale_factor,
            capture_ms: self.capture_ms,
            encode_ms: self.encode_ms,
            bytes: self.bytes.len(),
            shown_ms: self.shown_ms.unwrap_or_default(),
        }
    }
}

struct FrameSession {
    id: String,
    started: Instant,
    reconciliation_ms: f64,
    capture_total_ms: f64,
    entries: HashMap<String, FrameEntry>,
    metrics_emitted: bool,
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
        reconciliation_ms: f64,
        capture_total_ms: f64,
        entries: Vec<FrameEntry>,
    ) -> Result<(), String> {
        let mut current = self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())?;
        let entries = entries
            .into_iter()
            .map(|entry| (entry.monitor.label.clone(), entry))
            .collect();
        *current = Some(FrameSession {
            id,
            started,
            reconciliation_ms,
            capture_total_ms,
            entries,
            metrics_emitted: false,
        });
        Ok(())
    }

    pub fn pending(&self, label: &str) -> Option<PendingFrame> {
        let current = self.inner.lock().ok()?;
        let session = current.as_ref()?;
        let frame = session.entries.get(label)?;
        Some(PendingFrame {
            session: session.id.clone(),
            monitor: frame.monitor.clone(),
        })
    }

    pub fn frame_bytes(&self, label: &str, session_id: &str) -> Option<Vec<u8>> {
        let current = self.inner.lock().ok()?;
        let session = current.as_ref()?;
        if session.id != session_id {
            return None;
        }
        session
            .entries
            .get(label)
            .map(|frame| frame.bytes.as_ref().clone())
    }

    pub fn mark_ready(
        &self,
        label: &str,
        session_id: &str,
    ) -> Result<Option<CaptureMetrics>, String> {
        let mut current = self
            .inner
            .lock()
            .map_err(|_| "Frame state is unavailable".to_string())?;
        let Some(session) = current.as_mut() else {
            return Ok(None);
        };
        if session.id != session_id {
            return Ok(None);
        }
        let Some(frame) = session.entries.get_mut(label) else {
            return Ok(None);
        };
        if frame.shown_ms.is_none() {
            frame.shown_ms = Some(session.started.elapsed().as_secs_f64() * 1000.0);
        }
        if session.metrics_emitted
            || session
                .entries
                .values()
                .any(|entry| entry.shown_ms.is_none())
        {
            return Ok(None);
        }
        session.metrics_emitted = true;
        let mut monitors: Vec<_> = session.entries.values().map(FrameEntry::metrics).collect();
        monitors.sort_by(|a, b| a.label.cmp(&b.label));
        Ok(Some(CaptureMetrics {
            session: session.id.clone(),
            reconciliation_ms: session.reconciliation_ms,
            capture_total_ms: session.capture_total_ms,
            monitors,
        }))
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
