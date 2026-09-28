//! Minimal error log: every line goes to stderr and to `<app local data>/logs/keepshot.log`,
//! because installed builds have no console to read `eprintln!` output from.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Manager};

const MAX_LOG_BYTES: u64 = 1024 * 1024;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// Logs an error line; usable anywhere in the crate like `eprintln!`.
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::applog::write("ERROR", &format!($($arg)*))
    };
}

pub fn init(app: &AppHandle) {
    let Ok(directory) = app.path().app_local_data_dir().map(|dir| dir.join("logs")) else {
        return;
    };
    if fs::create_dir_all(&directory).is_ok() {
        let _ = LOG_PATH.set(directory.join("keepshot.log"));
    }
}

pub fn write(level: &str, message: &str) {
    let line = format!(
        "{} {level} {message}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f")
    );
    eprintln!("{line}");
    let Some(path) = LOG_PATH.get() else {
        return;
    };
    let Ok(_guard) = WRITE_LOCK.lock() else {
        return;
    };
    if fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_LOG_BYTES) {
        let _ = fs::rename(path, path.with_extension("log.old"));
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}
