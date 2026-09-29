# KeepShot

A lightweight, open source screenshot tool — a modern replacement for Lightshot.

- **Local history** of recent captures (no cloud).
- **Re-editable annotations**: reopen any capture from history and keep editing its layers.
- **Solid multi-monitor support**, including mixed DPI scaling and negative coordinates.

Windows and Linux. Built with [Tauri 2](https://tauri.app), Rust, Svelte and TypeScript.

**[Download for Windows and Linux](https://saggassa.github.io/keepshot/)** · [Releases](https://github.com/SaGgaSsa/keepshot/releases)

> Status: early development.

## Installation

Download KeepShot from the [release page](https://github.com/SaGgaSsa/keepshot/releases/latest).

### Windows

Run the NSIS installer. On Windows 11, Print Screen opens Snipping Tool by default; turn this off in Settings > Accessibility > Keyboard to use it for KeepShot.

### Linux

Choose the x86_64 AppImage, `.deb`, or `.rpm` package. KeepShot needs a recent distribution with PipeWire 1.0+ (for example Ubuntu 24.04, Debian 13, Fedora 40 or newer). To run the AppImage:

```sh
chmod +x KeepShot_*.AppImage
./KeepShot_*.AppImage
```

For a system tray icon on GNOME, install and enable an AppIndicator extension. On Wayland, configure capture and history shortcuts in the desktop keyboard settings using the commands shown in KeepShot Settings (`--capture` and `--history`). The first screen capture in GNOME Wayland may ask for permission through the screen capture portal.

## Development

Requirements: Rust (stable), Node.js 22+, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```sh
npm install
npm run tauri dev
```

Checks:

```sh
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## Design

The visual direction lives in [`docs/design/DESIGN.md`](docs/design/DESIGN.md).

## License

[MIT](LICENSE)
