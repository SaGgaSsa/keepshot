# Releasing KeepShot

## Automated release (default)

Every push to `main` runs the CI workflow (`.github/workflows/ci.yml`): Svelte/TypeScript checks,
`cargo fmt` and `cargo clippy`. If the version in `src-tauri/tauri.conf.json` has no matching
`vX.Y.Z` tag yet, CI also builds the signed NSIS installer with `tauri-apps/tauri-action`, creates
the tag and GitHub release on that commit, and uploads the installer, its `.sig` and `latest.json`.

To ship a new version:

1. Bump the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json`.
2. Write release notes in `.github/release-notes/vX.Y.Z.md` (optional; a generic note is used otherwise).
3. Merge to `main` and push. CI publishes the release; pushes without a version bump only run checks.

The workflow needs two repository secrets (Settings › Secrets and variables › Actions):

- `TAURI_SIGNING_PRIVATE_KEY`: the full contents of `keepshot-updater.key`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the contents of `keepshot-updater.password`.

The manual steps below remain as a fallback when CI is unavailable.

## Signing prerequisites

Keep the updater signing private key and its password outside the repository:

- `%USERPROFILE%\.tauri\keepshot-updater.key`
- `%USERPROFILE%\.tauri\keepshot-updater.password`

Never commit either file or their contents. The matching public key is configured in
`src-tauri/tauri.conf.json` and is safe to distribute with the application.

## Build a signed Windows release

From the repository root in PowerShell, load the signing credentials and build the NSIS installer:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw "$env:USERPROFILE\.tauri\keepshot-updater.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Get-Content -Raw "$env:USERPROFILE\.tauri\keepshot-updater.password"
npm run tauri build
npm run release:latest-json -- notes.md
```

The manifest generator reads the version from `src-tauri/tauri.conf.json`, the signature from
`src-tauri/target/release/bundle/nsis/KeepShot_<version>_x64-setup.exe.sig`, and release notes from
the optional file argument. It writes `src-tauri/target/release/bundle/latest.json` and checks that
the installer and signature exist first.

## Publish the release

Update the version consistently in `src-tauri/Cargo.toml`, `package.json`, and
`src-tauri/tauri.conf.json`. Build the signed installer and manifest, then publish a versioned
GitHub release with all three assets:

```powershell
gh release create vX.Y.Z `
  "src-tauri/target/release/bundle/nsis/KeepShot_X.Y.Z_x64-setup.exe" `
  "src-tauri/target/release/bundle/nsis/KeepShot_X.Y.Z_x64-setup.exe.sig" `
  "src-tauri/target/release/bundle/latest.json" `
  --title "KeepShot X.Y.Z" `
  --notes-file notes.md
```

The configured updater endpoint downloads `latest.json` from the latest GitHub release. Keep the
manifest's installer URL versioned to match the release tag and attached installer.

## End-to-end update check

Publish a signed release with a version greater than the installed build. Install the older signed
build, launch KeepShot, and use **Settings → Updates → Check for updates** or the tray update item.
Confirm the available version and release notes, start **Install and restart**, and verify that the
new version starts. Also check an installation already on the latest version and confirm it reports
that it is up to date. For local development, the manual check command works in debug builds; the
automatic six-hour check runs only in release builds.
