# Changelog

All notable changes to ShareCursor are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows
[Semantic Versioning](https://semver.org/). See
[docs/RELEASING.md](./docs/RELEASING.md) for the release process.

## [Unreleased]

### Fixed
- Fresh installations now create the per-user configuration before starting
  pairing, with platform-appropriate machine names and unique pairing codes.
- First launch opens Settings so setup is visible on macOS and Windows;
  pairing starts after the window closes, using the saved passphrase.
- macOS wakes the native run loop after creating the menu-bar icon so it
  appears immediately. The settings window activates in the foreground.
- Repeated Start clicks no longer create duplicate pairing workers, and
  background startup failures are shown in the tray menu.

## [0.6.0] - 2026-08-29

First stable release under the ShareCursor name.

### Added
- **Privacy-friendly website analytics** — Umami pageview tracking across the
  GitHub Pages site, with separate release-page click events for macOS, Windows,
  guides, and comparison pages.
- **Public privacy and support pages** — documents local app data handling,
  cookieless website analytics, support routes, and private vulnerability
  reporting.
- **Machine-readable website content** — Markdown mirrors, `llms.txt`, and a
  deterministic `llms-full.txt` generated and checked by CI.
- **Square brand assets** — web manifest icons at 192 and 512 pixels plus a
  1024-pixel app-listing source image.
- **Search-focused website guides** — an evidence-based 2026 Synergy alternatives
  comparison and a Mac-to-Windows clipboard-sharing guide, plus stronger homepage
  targeting, internal links, metadata, and current sitemap dates.
- **Symmetric control (ShareMouse-style)** — both machines are equals: use
  EITHER machine's mouse & keyboard, whichever you grab. Exactly one pointer is
  "away" at a time; the visited machine's real cursor is driven directly, so
  crossings feel like real adjacent monitors. (Protocol v4.)
- **Zero-config auto-pairing** — both machines advertise + search on the LAN
  and connect automatically: no IPs, no role picking (`sharecursor pair`, and
  the default when no role is set).
- **One-machine layout setup** — the monitor arrangement is exchanged in the
  handshake; the other machine adopts the mirrored layout automatically.
- **Real-monitor overlap edges** — crossing only happens where the two screens
  actually overlap (with the arrangement offset); elsewhere the edge is a wall.
- **Fully dynamic screen sizes** — always live-detected, reported in the
  handshake, never typed by hand.
- **Background service** — `sharecursor install-service` auto-starts ShareCursor
  on login (macOS LaunchAgent / Windows startup), no terminal needed.
- **Server/Client/auto role selector** + no-overlap monitor arrangement UI.
- **Brand icon** everywhere: tray, settings window, site favicon, macOS .app
  icon and the Windows .exe/installer icon.
- **Seamless macOS cursor switching** — while the client has control the local
  Mac cursor is hidden and pinned (Deskflow technique: `SetsCursorInBackground`
  + warp-to-centre + live position), so the pointer truly *leaves* one screen
  and appears on the other instead of mirroring.
- **Visual settings & monitor-arrangement window** (`gui` feature) — drag the two
  monitors to lay them out like macOS Displays; ShareCursor computes the edge
  adjacency and saves the config. Opened from the tray **Settings**.
- **Automatic remote screen size** — the client reports its resolution on connect
  (like Deskflow's DINF), so the arrangement window shows the real size.

### Changed
- **Project identity and website domain** — renamed the application, executables,
  packages, installers, documentation, and repository references to ShareCursor.
  The canonical website URL is now `https://sharecursor.com/`.
- **Development status and contribution guidance** — the website, README, and
  documentation now explain that compatibility may vary during active
  development and link to bug reports, feedback, and pull requests. GitHub issue
  forms now ask only for the details needed to start triage.
- **Release packaging** — standardized the application and installer version at
  0.6.0, adopted the permanent `com.sharecursor.app` macOS bundle identifier,
  and added generated SHA-256, Scoop, and Homebrew release metadata.

### Fixed
- mDNS advertisements now include the stable device ID used to identify peers.
- Stuck modifier keys (Ctrl/Alt "Alt+Tab" bug) after a control hand-off — all
  modifiers are now released on every switch.
- Windows: hide the stray console window when launched as the tray app.
- Cursor-return edge is now the opposite of the exit edge (fixes not being able
  to return control).

## [0.1.0] - 2026-07-06

First release. A complete, low-latency, open-source software KVM.

### Added
- **Input sharing** — one keyboard & mouse across macOS ⇄ Windows, via rdev
  capture + enigo injection, with a portable cross-platform key mapping.
- **Low-latency transport** — hybrid UDP (input) + TCP (bulk) with sequence
  numbers and per-tick event coalescing; ~6 µs one-way loopback overhead.
- **Real control handoff** — rdev `grab` swallows local input while the client
  has control; F12 toggle plus automatic screen-edge switching in both
  directions (client auto-returns via cursor tracking).
- **Encryption** — X25519 + pre-shared-key handshake + ChaCha20-Poly1305 on both
  channels; measured ~20 ns latency cost.
- **Clipboard sync** — text and images (raw RGBA), with echo suppression.
- **File transfer** — chunked, offset-based, path-traversal-safe (`send-file`,
  received into `./received/`).
- **Settings + monitor manager** — TOML config describing the machine/edge
  layout, PSK, and port (`init-config`).
- **mDNS discovery** — zero-config peer finding (`discover`, or `connect` with no
  host).
- **GUI** — macOS menu-bar status item / Windows system tray (`--features tray`).
- **Installers** — universal macOS `.dmg` and Windows `.exe`, built and published
  by CI on tag. No Rust required for end users.
- **Docs** — full `docs/` set (architecture, protocol, security, development,
  releasing, decisions, history).

### Known limitations
- Builds are unsigned (Gatekeeper/SmartScreen prompt on first launch).
- One client at a time; no sliding-window UDP anti-replay yet.

[Unreleased]: https://github.com/phun333/ShareCursor/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/phun333/ShareCursor/compare/v0.1.0...v0.6.0
[0.1.0]: https://github.com/phun333/ShareCursor/releases/tag/v0.1.0
