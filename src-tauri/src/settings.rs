use std::str::FromStr;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_store::StoreExt;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub capture_shortcut: String,
    pub history_shortcut: String,
    pub save_folder: Option<String>,
    pub onboarding_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            capture_shortcut: "PrintScreen".to_string(),
            history_shortcut: "Ctrl+PrintScreen".to_string(),
            save_folder: None,
            onboarding_done: false,
        }
    }
}

pub struct SettingsState {
    pub values: Mutex<Settings>,
    pub capture: Mutex<Option<Shortcut>>,
    pub history: Mutex<Option<Shortcut>>,
    pub shortcut_errors: Mutex<Vec<String>>,
    pub suspended: Mutex<bool>,
    pub window_material: Mutex<String>,
    /// Set once the Settings page has painted; showing the window earlier flashes white.
    pub page_ready: Mutex<bool>,
    pub show_pending: Mutex<bool>,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            values: Mutex::new(Settings::default()),
            capture: Mutex::new(None),
            history: Mutex::new(None),
            shortcut_errors: Mutex::new(Vec::new()),
            suspended: Mutex::new(false),
            window_material: Mutex::new("solid".to_string()),
            page_ready: Mutex::new(false),
            show_pending: Mutex::new(false),
        }
    }
}

const PRINT_SCREEN_MIGRATION: &str = "migratedToPrintScreen";

pub fn load(app: &AppHandle) -> Result<Settings, String> {
    let store = app
        .store("settings.json")
        .map_err(|error| format!("Could not open settings store: {error}"))?;
    let mut settings: Settings = store
        .get("settings")
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    // Up to 0.1.1 the default capture shortcut was Ctrl+Shift+X and completing onboarding
    // persisted it, so move those installs to Print Screen once. A marker keeps a later,
    // deliberate choice of Ctrl+Shift+X from being migrated again.
    if store.get(PRINT_SCREEN_MIGRATION).is_none() {
        if settings.capture_shortcut == "Ctrl+Shift+X" {
            settings.capture_shortcut = "PrintScreen".to_string();
            let value = serde_json::to_value(&settings).map_err(|error| error.to_string())?;
            store.set("settings", value);
        }
        store.set(PRINT_SCREEN_MIGRATION, true);
        store
            .save()
            .map_err(|error| format!("Could not save settings: {error}"))?;
    }
    let defaults = Settings::default();
    if shortcut(&settings.capture_shortcut).is_err() {
        settings.capture_shortcut = defaults.capture_shortcut.clone();
    }
    if shortcut(&settings.history_shortcut).is_err()
        || shortcut(&settings.history_shortcut).ok() == shortcut(&settings.capture_shortcut).ok()
    {
        let alternate = "Ctrl+Alt+PrintScreen";
        let alternate_parsed = shortcut(alternate).ok();
        settings.history_shortcut = if alternate_parsed == shortcut(&settings.capture_shortcut).ok()
        {
            "Ctrl+Shift+Alt+PrintScreen".to_string()
        } else {
            alternate.to_string()
        };
    }
    Ok(settings)
}

pub fn persist(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let store = app
        .store("settings.json")
        .map_err(|error| format!("Could not open settings store: {error}"))?;
    let value = serde_json::to_value(settings).map_err(|error| error.to_string())?;
    store.set("settings", value);
    store
        .save()
        .map_err(|error| format!("Could not save settings: {error}"))
}

pub fn snapshot(app: &AppHandle) -> Result<Settings, String> {
    app.state::<SettingsState>()
        .values
        .lock()
        .map_err(|_| "Settings are unavailable".to_string())
        .map(|settings| settings.clone())
}

pub fn shortcut(value: &str) -> Result<Shortcut, String> {
    Shortcut::from_str(value).map_err(|error| format!("Invalid shortcut: {error}"))
}

pub fn update(app: &AppHandle, settings: Settings) -> Result<(), String> {
    persist(app, &settings)?;
    *app.state::<SettingsState>()
        .values
        .lock()
        .map_err(|_| "Settings are unavailable".to_string())? = settings;
    Ok(())
}

pub fn register_startup_shortcuts(app: &AppHandle) {
    if !crate::platform::global_shortcuts_supported() {
        if let Ok(mut errors) = app.state::<SettingsState>().shortcut_errors.lock() {
            errors.clear();
        }
        return;
    }
    let current = match snapshot(app) {
        Ok(settings) => settings,
        Err(error) => {
            crate::log_error!("Could not read settings: {error}");
            return;
        }
    };
    let manager = app.global_shortcut();
    let state = app.state::<SettingsState>();
    let mut errors = Vec::new();
    for (name, value, slot) in [
        ("capture", current.capture_shortcut.as_str(), "capture"),
        ("history", current.history_shortcut.as_str(), "history"),
    ] {
        let result = shortcut(value).and_then(|parsed| {
            manager
                .register(parsed)
                .map(|_| parsed)
                .map_err(|error| {
                    let message = registration_error(&error.to_string());
                    #[cfg(target_os = "linux")]
                    if message == "That shortcut is already in use by another app"
                        && value.to_ascii_lowercase().contains("printscreen")
                    {
                        return format!("{message}. Your desktop may already use Print Screen for its own screenshot tool; unbind it in your keyboard settings or pick another shortcut.");
                    }
                    message
                })
        });
        match result {
            Ok(parsed) if slot == "capture" => {
                if let Ok(mut current) = state.capture.lock() {
                    *current = Some(parsed);
                }
            }
            Ok(parsed) => {
                if let Ok(mut current) = state.history.lock() {
                    *current = Some(parsed);
                }
            }
            Err(error) => {
                crate::log_error!("Could not register {name} shortcut: {error}");
                errors.push(format!("{name}: {error}"));
            }
        }
    }
    if let Ok(mut current) = state.shortcut_errors.lock() {
        *current = errors;
    };
}

pub fn registration_error(error: &str) -> String {
    let normalized = error.to_ascii_lowercase().replace([' ', '_'], "");
    if normalized.contains("alreadyregistered") || normalized.contains("alreadyinuse") {
        "That shortcut is already in use by another app".to_string()
    } else {
        format!("Could not register shortcut: {error}")
    }
}
