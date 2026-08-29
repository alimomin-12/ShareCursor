<div align="center">

<img src="https://raw.githubusercontent.com/phun333/ShareCursor/main/site/sharecursor-og.png" alt="ShareCursor — a low-latency, open-source software KVM for macOS and Windows" width="100%" />

<h1>ShareCursor</h1>

**A low-latency, open-source software KVM.** Move one keyboard & mouse — plus the
clipboard and files — between your **macOS** and **Windows** machines over the
LAN, end-to-end encrypted, with the lowest input lag we can squeeze out.

[![Download](https://img.shields.io/badge/Download-2563eb?style=for-the-badge)](https://github.com/phun333/ShareCursor/releases)
[![Docs](https://img.shields.io/badge/Docs-1d4ed8?style=for-the-badge)](https://docs.sharecursor.com)
[![Website](https://img.shields.io/badge/Website-60a5fa?style=for-the-badge)](https://sharecursor.com/)

![License](https://img.shields.io/badge/license-MIT_%2F_Apache--2.0-green)
![Platforms](https://img.shields.io/badge/platforms-macOS_%7C_Windows-lightgrey)
![Latency](https://img.shields.io/badge/transport-~6µs_one--way-blueviolet)

</div>

---

> **Development status:** ShareCursor is under active development and may not
> work reliably on every macOS and Windows setup. If you try it, please
> [report bugs](https://github.com/phun333/ShareCursor/issues/new?template=bug_report.yml),
> [suggest improvements](https://github.com/phun333/ShareCursor/issues/new?template=feature_request.yml), or
> [contribute a fix](./CONTRIBUTING.md).

A free **alternative to Synergy, ShareMouse, Barrier and Input Leap**, the
Mac-capable answer to **Mouse Without Borders**, and effectively **Universal
Control for Windows**.

- **One keyboard & mouse** — push your cursor across the screen edge to control the other machine.
- **Shared clipboard** — copy on one machine, paste on the other (text + images).
- **Drag-and-drop files** — reliable, chunked transfer between machines.
- **End-to-end encrypted** — X25519 + a shared passphrase + ChaCha20-Poly1305. LAN-only, no cloud, no accounts.
- **Latency-first** — dedicated UDP input path, ~6 µs one-way transport overhead.

## Install

**macOS (Homebrew)**
```bash
brew install --cask phun333/tap/sharecursor
```

**Windows (Scoop)**
```powershell
scoop install https://github.com/phun333/ShareCursor/releases/latest/download/sharecursor.json
```

Or grab the installer from the [**Releases**](https://github.com/phun333/ShareCursor/releases) page.

> **Note:** builds are **unsigned** (no paid Apple/Microsoft certificate), so your OS
> shows a one-time warning. Follow the
> [installation docs](https://docs.sharecursor.com/installation) to review and
> allow the app.

## Try ShareCursor

On **both** machines: open ShareCursor (menu-bar on macOS, system tray on
Windows) → **Settings & Monitor Manager** → set the **same passphrase** and your
screen layout → save. Then push your mouse into the shared screen edge to jump
across; copy/paste and files sync automatically.

**Full walkthrough in the [Quickstart](https://docs.sharecursor.com/quickstart).**

## How it compares

| Tool | License | Mouse/KB | Clipboard | Files |
|------|---------|:--------:|:---------:|:-----:|
| ShareMouse | Paid | Yes | Yes | Yes |
| Synergy | Paid | Yes | Yes | Yes |
| Deskflow | GPLv2 | Yes | Yes | Partial |
| Input Leap | GPLv2 | Yes | Yes | Partial |
| Barrier | Open (archived) | Yes | Yes | No |
| Lan Mouse | Open | Yes | No | No |
| **ShareCursor** | **MIT/Apache** | Yes | Yes | Yes |

See the full [comparisons](https://docs.sharecursor.com/compare/overview).

## Documentation

Everything lives in the **[docs site](https://docs.sharecursor.com)** —
install, usage, configuration, troubleshooting, architecture, wire protocol,
security model, and contributor guides.

Building from source? See the [development guide](https://docs.sharecursor.com/develop/development).
The Mintlify source lives in [`mintlify/`](./mintlify/README.md).

## Contributing

PRs welcome! Read [CONTRIBUTING.md](./CONTRIBUTING.md) first — we use
[Conventional Commits](https://www.conventionalcommits.org/), and a one-time
`./scripts/setup-hooks.sh` installs git hooks that keep CI green.

## License

Dual-licensed under [**MIT**](./LICENSE-MIT) or [**Apache-2.0**](./LICENSE-APACHE) — your choice.
