# KeepShot

A lightweight, open source screenshot tool — a modern replacement for Lightshot.

- **Local history** of recent captures (no cloud).
- **Re-editable annotations**: reopen any capture from history and keep editing its layers.
- **Solid multi-monitor support**, including mixed DPI scaling and negative coordinates.

Windows first; Linux planned. Built with [Tauri 2](https://tauri.app), Rust, Svelte and TypeScript.

> Status: early development. Nothing to install yet.

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
`docs/design/stitch-reference.html` is a mockup exported from Google Stitch — a visual reference only, not production code.

## License

[MIT](LICENSE)
