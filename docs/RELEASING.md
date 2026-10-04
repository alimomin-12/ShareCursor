# Releasing

This is the exact, repeatable process to ship a new version. Following it means
end users always get working one-click installers and the history stays clean.

## Versioning policy (SemVer)

`MAJOR.MINOR.PATCH`:

- **PATCH** — bug fixes, no wire or config changes.
- **MINOR** — new features, backward-compatible wire/config (old configs still
  load thanks to `#[serde(default)]`; `PROTOCOL_VERSION` unchanged).
- **MAJOR** — breaking wire/config changes → also bump `PROTOCOL_VERSION` in
  `crates/protocol/src/lib.rs` and document the migration in the changelog.

The crate version lives in `Cargo.toml` under `[workspace.package]`; all crates
inherit it. The Windows installer default, website schema, lockfile, and
changelog mirror that version. Run `node scripts/check-release-version.mjs
X.Y.Z` before tagging to verify every release surface.

## Release checklist

1. **Green build & tests**
   ```bash
   cargo test                       # native
   cargo test --no-default-features # core
   cargo build --release --features tray,gui
   cargo run --release -- bench --encrypted   # no latency regression
   ```
2. **Bump the version** in `Cargo.toml`, the Windows installer default, and the
   website `SoftwareApplication` schema. Run `cargo build` once so `Cargo.lock`
   updates, then run:
   ```bash
   node scripts/check-release-version.mjs X.Y.Z
   ```
3. **Update [CHANGELOG.md](../CHANGELOG.md):** move `## [Unreleased]` items under
   a new `## [X.Y.Z] - YYYY-MM-DD` heading; start a fresh empty `Unreleased`.
4. **Commit** on `main`:
   ```bash
   git add -A && git commit -m "release: vX.Y.Z"
   ```
5. **Tag & push** — this triggers the CI release workflow:
   ```bash
   git tag vX.Y.Z
   git push origin main --tags
   ```
6. **Wait for CI.** `.github/workflows/release.yml` builds and attaches:
   - `ShareCursor-X.Y.Z.dmg` (macOS universal, arm64 + Intel)
   - `ShareCursor-Setup-X.Y.Z.exe` (Windows installer)
   - `ShareCursor-SHA256SUMS.txt`
   - `sharecursor.json` (generated Scoop manifest)
   - `sharecursor.rb` (generated Homebrew cask)
7. **Smoke-test the artifacts** on both OSes (install, launch, connect once) and
   verify the published checksums.
8. **Update package repositories** after smoke testing: copy `sharecursor.json`
   to the Scoop bucket and `sharecursor.rb` to `Casks/sharecursor.rb` in the
   Homebrew tap. These external repositories are not changed automatically.
9. **Announce** — the Release page is the download link for users.

## What CI does (`.github/workflows/release.yml`)

- Trigger: pushing a tag matching `v*`, manual `workflow_dispatch`, or a pull
  request changing application or packaging code. Pull requests test the
  startup and GUI features and upload both installers without publishing a
  release; download them from the workflow run's Artifacts section.
- `macos` job (macos-14): adds both Apple targets, runs
  `packaging/macos/build-app.sh $VERSION` → universal `.app` + `.dmg`.
- `windows` job (windows-latest): `cargo build --release --features tray,gui`, then
  `choco install innosetup` and `ISCC /DMyAppVersion=$VERSION sharecursor.iss` →
  `.exe` installer.
- `release` job: generates checksums plus Scoop/Homebrew manifests from the
  built artifacts, then publishes all files in the GitHub Release.

## Building installers locally (optional)

```bash
# macOS (produces dist/ShareCursor.app + dist/ShareCursor-<ver>.dmg)
bash packaging/macos/build-app.sh 0.6.0

# Windows (in a Windows shell, after cargo build --release --features tray,gui)
iscc /DMyAppVersion=0.6.0 packaging\windows\sharecursor.iss
```

## Code signing & notarization (current status)

Builds are **unsigned** today (no paid developer certificates), so:

- **macOS:** Gatekeeper blocks first launch. Users right-click the app → **Open**
  once. To remove this friction we would need an Apple Developer ID ($99/yr) and
  a notarization step in the macOS CI job (`codesign` with the Developer ID +
  `xcrun notarytool submit`). The build script already ad-hoc signs so it runs
  locally.
- **Windows:** SmartScreen shows an "unknown publisher" warning; users click
  **More info → Run anyway**. An EV/OV code-signing certificate would remove it.

When certificates are available, add the signing secrets to the repository and
explicit signing and notarization steps to the CI jobs.

## Rollback

If a release is broken, delete the GitHub Release + tag, fix, and re-tag with a
new PATCH version. Never re-use a published version number.
