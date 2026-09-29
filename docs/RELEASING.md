# Releasing KeepShot

## Automated release (default)

Every push to `main` runs Windows and Linux checks. When the version in `src-tauri/tauri.conf.json` has no matching `vX.Y.Z` tag, CI prepares a draft release, builds both platforms, uploads the artifacts, and publishes it only after every build succeeds. The same GitHub release contains the Windows NSIS installer and Linux AppImage, `.deb`, and `.rpm` packages, along with updater signatures and the merged `latest.json` manifest.

To ship a new version:

1. Bump the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json`.
2. Write release notes in `.github/release-notes/vX.Y.Z.md` (optional; a generic note is used otherwise).
3. Merge to `main` and push.

The workflow needs two repository secrets (Settings > Secrets and variables > Actions):

- `TAURI_SIGNING_PRIVATE_KEY`: the full contents of `keepshot-updater.key`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the contents of `keepshot-updater.password`.

Keep both files outside the repository (`%USERPROFILE%\.tauri\` on the release machine) and never
commit them. The matching public key in `src-tauri/tauri.conf.json` is safe to distribute. The same
key signs the Windows and Linux updater artifacts.

The Linux updater selects `linux-x86_64-appimage`, `linux-x86_64-deb`, or `linux-x86_64-rpm` based on the installed package. Updating `.deb` and `.rpm` installations asks for the user's password through `pkexec`; AppImage updates replace the AppImage in place.

## Local Linux build

On Ubuntu 24.04 (or newer; xcap needs PipeWire 1.0+), install the CI build dependencies and compile all Linux formats:

```sh
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libxdo-dev libxcb1-dev libxcb-randr0-dev libxrandr-dev libdbus-1-dev libpipewire-0.3-dev libspa-0.2-dev libclang-dev clang libgbm-dev libegl-dev libwayland-dev pkg-config rpm file
npm ci
npm run tauri build -- --bundles appimage,deb,rpm
```

## Signed Windows build

From the repository root in PowerShell:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw "$env:USERPROFILE\.tauri\keepshot-updater.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Get-Content -Raw "$env:USERPROFILE\.tauri\keepshot-updater.password"
npm run tauri build -- --bundles nsis
npm run release:latest-json -- notes.md
```

The manual manifest generator remains a Windows fallback. CI creates and publishes releases automatically.

## End-to-end update check

Publish a signed release with a version greater than the installed build. Check for updates from Settings or the tray, install and restart, then verify the new version starts. For `.deb` and `.rpm`, confirm the system requests authorization through `pkexec`.
