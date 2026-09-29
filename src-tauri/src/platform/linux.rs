use std::path::Path;

pub fn is_wayland() -> bool {
    std::env::var_os("XDG_SESSION_TYPE").is_some_and(|value| value == "wayland")
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub fn prepare_environment() {
    // This runs before Tauri creates its GTK/WebKit threads.
    if is_wayland() && std::env::var_os("GDK_BACKEND").is_none() {
        std::env::set_var("GDK_BACKEND", "x11");
    }
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

pub fn launch_command(arg: &str) -> Option<String> {
    let executable = std::env::var_os("APPIMAGE")
        .filter(|path| !path.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok())?;
    Some(format!("{} {arg}", shell_quote_path(&executable)))
}

fn shell_quote_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    if path.chars().any(char::is_whitespace) {
        format!("\"{}\"", path.replace('"', "\\\""))
    } else {
        path.into_owned()
    }
}
