use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn snipping_tool_owns_print_screen() -> Option<bool> {
    // Query the whole key and look the value up by name: reg.exe error messages are localized,
    // so "value not found" cannot be detected reliably from its output.
    let output = Command::new("reg.exe")
        .args(["query", r"HKCU\Control Panel\Keyboard"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(line) = text.lines().find(|line| {
        line.trim_start()
            .starts_with("PrintScreenKeyForSnippingEnabled")
    }) else {
        // Missing value: up-to-date Windows 11 defaults to Snipping Tool owning Print Screen.
        return Some(true);
    };
    let value = line.split_whitespace().last()?;
    match u32::from_str_radix(value.trim_start_matches("0x"), 16) {
        Ok(0) => Some(false),
        Ok(_) => Some(true),
        Err(_) => None,
    }
}

pub fn open_keyboard_settings() -> Result<(), String> {
    Command::new("explorer")
        .arg("ms-settings:easeofaccess-keyboard")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open Windows keyboard settings: {error}"))
}
