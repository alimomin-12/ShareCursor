# ShareCursor vs Input Leap

> Input Leap was archived in July 2026. Compare its platforms, clipboard support
> and maintenance status with the free, open-source ShareCursor software KVM.

Input Leap was the community-maintained fork of Barrier, supporting keyboard,
mouse and clipboard sharing across macOS, Windows, Linux and BSD. Its GitHub
repository was **archived on July 26, 2026** and is now read-only. ShareCursor is
an actively developed, free alternative focused on encrypted **Mac-to-Windows**
sharing.

Status checked against the [official Input Leap repository](https://github.com/input-leap/input-leap)
on **August 22, 2026**.

| Feature | ShareCursor | Input Leap |
| --- | --- | --- |
| Price | Free & open source | Free & open source |
| Project status | Active, pre-release | Archived July 2026 |
| Encryption | **On by default (X25519 + ChaCha20)** | TLS available |
| Clipboard | Text + images | Supported; not on Linux/Wayland |
| File transfer | **Yes** | No |
| Discovery | **mDNS (no IPs)** | Bonjour or manual server address |
| Input transport | **UDP, ~6 µs** | TCP |
| Linux | Work in progress | Yes |
| Mac & Windows | Yes | Yes |

## Which should you pick?

- **Choose ShareCursor** for **Mac-to-Windows** if you want encryption on by
  default, file transfer, clipboard images and mDNS discovery, and pre-release
  software is acceptable.
- **Keep Input Leap** if an existing installation works for you and its archived
  status is acceptable. For a new Linux deployment, evaluate an actively
  maintained option such as Deskflow or Lan Mouse.

## What does archived mean?

Archiving does not disable Input Leap or remove its releases. It means the
repository is read-only, so users should not expect fixes for future
operating-system changes or newly reported security issues from that project.
This matters for software that captures keyboard input and reads clipboard data.

Input Leap and Barrier proved how useful an open-source software KVM can be.
ShareCursor is a newer, security-focused implementation for Mac and Windows: a
UDP input path (~6 µs transport overhead), encryption on every channel,
clipboard images, file transfer and mDNS discovery. ShareCursor is still
pre-release, so users who require Linux or a longer production history should
also evaluate Deskflow.

- [Download ShareCursor (free)](https://github.com/phun333/ShareCursor/releases)
- [Setup guide](https://sharecursor.com/how-to-share-mouse-keyboard-mac-windows.md)
- [2026 Synergy alternatives comparison](https://sharecursor.com/synergy-alternatives.md)
