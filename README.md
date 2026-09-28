# KeepShot

A lightweight, open source screenshot tool — a modern replacement for Lightshot.

- **Local history** of recent captures (no cloud).
- **Re-editable annotations**: reopen any capture from history and keep editing its layers.
- **Solid multi-monitor support**, including mixed DPI scaling and negative coordinates.

Windows first; Linux planned. Built with [Tauri 2](https://tauri.app), Rust, Svelte and TypeScript.

**[Download for Windows](https://saggassa.github.io/keepshot/)** · [Releases](https://github.com/SaGgaSsa/keepshot/releases)

> Status: early development.

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
