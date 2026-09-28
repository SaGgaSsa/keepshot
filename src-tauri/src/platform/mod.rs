#[cfg(windows)]
mod windows;

pub fn supports_mica() -> bool {
    #[cfg(windows)]
    {
        windows::supports_mica()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn snipping_tool_owns_print_screen() -> Option<bool> {
    #[cfg(windows)]
    {
        windows::snipping_tool_owns_print_screen()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn open_keyboard_settings() -> Result<(), String> {
    #[cfg(windows)]
    {
        windows::open_keyboard_settings()
    }
    #[cfg(not(windows))]
    {
        Err("Keyboard settings are not available on this platform".to_string())
    }
}

pub fn open_folder(path: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    let result = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(path).spawn();
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(path).spawn();
    result
        .map(|_| ())
        .map_err(|error| format!("Could not open folder: {error}"))
}
