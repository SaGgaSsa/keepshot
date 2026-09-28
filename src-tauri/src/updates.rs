use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
#[derive(Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    Available {
        version: String,
        notes: Option<String>,
        date: Option<String>,
    },
    Downloading {
        downloaded: u64,
        total: Option<u64>,
    },
    UpToDate,
    Error {
        message: String,
    },
}

#[derive(Default)]
pub struct UpdateState {
    status: Mutex<UpdateStatus>,
    last_checked: Mutex<Option<String>>,
    checking: Mutex<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusView {
    status: UpdateStatus,
    current_version: String,
    last_checked: Option<String>,
}

#[tauri::command]
pub fn get_update_status(app: AppHandle) -> Result<UpdateStatusView, String> {
    let state = app.state::<UpdateState>();
    let status = state
        .status
        .lock()
        .map_err(|_| "Update status is unavailable".to_string())?
        .clone();
    let last_checked = state
        .last_checked
        .lock()
        .map_err(|_| "Update status is unavailable".to_string())?
        .clone();
    Ok(UpdateStatusView {
        status,
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        last_checked,
    })
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateStatusView, String> {
    check(&app).await?;
    get_update_status(app)
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = match app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await
    {
        Ok(Some(update)) => update,
        Ok(None) => {
            set_status(&app, UpdateStatus::UpToDate, true);
            return Ok(());
        }
        Err(error) => {
            let message = error.to_string();
            if is_missing_manifest(&error) {
                set_status(&app, UpdateStatus::UpToDate, true);
                return Ok(());
            }
            set_status(
                &app,
                UpdateStatus::Error {
                    message: message.clone(),
                },
                true,
            );
            crate::log_error!("Update installation check failed: {message}");
            return Err(message);
        }
    };

    let progress_app = app.clone();
    let mut downloaded = 0_u64;
    if let Err(error) = update
        .download_and_install(
            move |chunk_len, content_len| {
                downloaded = downloaded.saturating_add(chunk_len as u64);
                set_status(
                    &progress_app,
                    UpdateStatus::Downloading {
                        downloaded,
                        total: content_len,
                    },
                    false,
                );
            },
            || {},
        )
        .await
    {
        let message = error.to_string();
        set_status(
            &app,
            UpdateStatus::Error {
                message: message.clone(),
            },
            true,
        );
        crate::log_error!("Update installation failed: {message}");
        return Err(message);
    }

    app.restart();
}

pub fn start_automatic(app: AppHandle) {
    #[cfg(not(debug_assertions))]
    tauri::async_runtime::spawn(async move {
        sleep_blocking(std::time::Duration::from_secs(15)).await;
        loop {
            if let Err(error) = check(&app).await {
                crate::log_error!("Automatic update check failed: {error}");
            }
            sleep_blocking(std::time::Duration::from_secs(6 * 60 * 60)).await;
        }
    });
    #[cfg(debug_assertions)]
    let _ = app;
}

pub fn available_version(app: &AppHandle) -> Option<String> {
    app.state::<UpdateState>()
        .status
        .lock()
        .ok()
        .and_then(|status| match &*status {
            UpdateStatus::Available { version, .. } => Some(version.clone()),
            _ => None,
        })
}

#[cfg(not(debug_assertions))]
async fn sleep_blocking(duration: std::time::Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(duration)).await;
}

async fn check(app: &AppHandle) -> Result<(), String> {
    {
        let state = app.state::<UpdateState>();
        let mut checking = state
            .checking
            .lock()
            .map_err(|_| "Update status is unavailable".to_string())?;
        if *checking {
            return Ok(());
        }
        *checking = true;
    }
    set_status(app, UpdateStatus::Checking, false);
    let response = match app.updater() {
        Ok(updater) => updater.check().await.map_err(|error| {
            if is_missing_manifest(&error) {
                None
            } else {
                Some(error.to_string())
            }
        }),
        Err(error) => Err(Some(error.to_string())),
    };
    let result = match response {
        Ok(Some(update)) => {
            let date = update.date.map(|value| value.to_string());
            set_status(
                app,
                UpdateStatus::Available {
                    version: update.version,
                    notes: update.body,
                    date,
                },
                true,
            );
            Ok(())
        }
        Ok(None) => {
            set_status(app, UpdateStatus::UpToDate, true);
            Ok(())
        }
        Err(None) => {
            set_status(app, UpdateStatus::UpToDate, true);
            Ok(())
        }
        Err(Some(message)) => {
            set_status(
                app,
                UpdateStatus::Error {
                    message: message.clone(),
                },
                true,
            );
            crate::log_error!("Update check failed: {message}");
            Err(message)
        }
    };
    if let Ok(mut checking) = app.state::<UpdateState>().checking.lock() {
        *checking = false;
    }
    result
}

/// A release without `latest.json` (e.g. published before the updater existed) is not an error:
/// the plugin reports it as `ReleaseNotFound`, whose message mentions neither 404 nor "not found".
fn is_missing_manifest(error: &tauri_plugin_updater::Error) -> bool {
    matches!(error, tauri_plugin_updater::Error::ReleaseNotFound)
        || error.to_string().contains("404")
}

fn set_status(app: &AppHandle, status: UpdateStatus, checked: bool) {
    let state = app.state::<UpdateState>();
    if let Ok(mut current) = state.status.lock() {
        *current = status.clone();
    }
    if checked {
        if let Ok(mut current) = state.last_checked.lock() {
            *current = Some(chrono::Local::now().to_rfc3339());
        }
    }
    if let Err(error) = app.emit_to("main", "updates:status", status) {
        crate::log_error!("Could not emit update status: {error}");
    }
    if let Err(error) = crate::tray::update_menu(app) {
        crate::log_error!("Could not refresh tray after update status changed: {error}");
    }
}
