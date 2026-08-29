# ShareCursor vs Synergy

> A free, open-source Synergy alternative to share mouse, keyboard, clipboard and
> files between Mac and Windows. Encrypted and designed for low input latency.

Looking for a **free, open-source Synergy alternative**? ShareCursor shares one
keyboard, mouse, clipboard and files between your Mac and Windows PC with no
license, encryption on every channel and a transport designed for low input
latency.

Compared against the [official Synergy product and support pages](https://symless.com/synergy)
on **August 22, 2026**.

| Feature | ShareCursor | Synergy |
| --- | --- | --- |
| Price | **Free** | Paid license |
| Open source | Yes (MIT / Apache) | Core (Deskflow) yes; app proprietary |
| Mac & Windows | Yes | Yes |
| Linux | Work in progress | Yes |
| Encryption | X25519 + ChaCha20-Poly1305 | TLS |
| Clipboard (text + images) | Yes | Yes |
| File transfer | Yes | Yes |
| Auto edge switching | Yes (+ hotkey) | Yes |
| Input latency focus | **~6 µs transport (UDP)** | Good |
| Support | Community / GitHub | Commercial support |

## Pricing and licensing

ShareCursor is released under MIT or Apache-2.0 and does not require a license
key. Synergy is a commercial product with active license requirements; the
number of computers and support options depend on the purchased edition.
Symless changed parts of its licensing policy in 2026, so buyers should use the
current official license and pricing pages rather than an old third-party
comparison.

The practical decision is not simply free versus paid. A Synergy license funds a
mature commercial product and support organization. ShareCursor provides source
access and no usage fee, but support is community-based and the project is still
pre-release.

## Platform support and setup

Synergy supports Windows, macOS and Linux and has a long production history.
ShareCursor currently focuses on macOS and Windows. Both products install an
application on every computer and switch control when the pointer reaches a
configured screen edge. ShareCursor uses mDNS discovery and exchanges the monitor
arrangement between paired devices; Synergy 3 also advertises automatic
configuration and discovery.

Choose Synergy when Linux, older deployment experience or paid support is
mandatory. Choose ShareCursor when the desk is specifically Mac plus Windows and
open-source licensing is a core requirement.

## Clipboard, files and security

Both products cover the main software KVM workflow: keyboard and mouse control,
clipboard sharing and file movement. ShareCursor synchronizes text and images and
sends files over a separate reliable channel. Its pairing uses X25519 key
exchange with ChaCha20-Poly1305 on both input and bulk traffic. Synergy uses TLS
for encrypted network communication.

Neither product shares the display itself. Each computer keeps using its own
monitor. If you need BIOS access or need several computers to share one physical
monitor, use a hardware KVM instead.

## What the latency number means

ShareCursor's approximately 6 µs figure is measured transport overhead on a local
loopback benchmark. It is not a full Mac-to-Windows latency measurement and is
not a direct benchmark against Synergy. Real input latency also includes capture,
network, scheduling, display refresh and input injection. The architectural
difference is factual: ShareCursor uses a dedicated UDP input path while reliable
clipboard and file traffic stays separate.

## Which should you pick?

- **Choose ShareCursor** if you want it **free and open source**, encrypted by
  default, with no license to manage.
- **Choose Synergy** if you need first-class **Linux** support and paid
  commercial support, and do not mind the license fee.

Synergy is the mature commercial option with broad platform support. Deskflow
carries the open-source continuation of the Synergy 1 codebase. ShareCursor is a
separate, newer implementation focused on a free and encrypted Mac-to-Windows
workflow.

## Moving from Synergy to ShareCursor

There is no configuration import because the projects use different formats and
protocols. Install ShareCursor on both computers, pair them, arrange the displays
and confirm macOS permissions and the Windows firewall rule. Keep Synergy
installed until the new setup has passed normal keyboard, clipboard and
file-transfer tests.

- [Download ShareCursor (free)](https://github.com/phun333/ShareCursor/releases)
- [Setup guide](https://sharecursor.com/how-to-share-mouse-keyboard-mac-windows.md)
- [Synergy alternatives compared](https://sharecursor.com/synergy-alternatives.md)
