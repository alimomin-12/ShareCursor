# ShareCursor vs Mouse Without Borders

> Compare ShareCursor with Microsoft Mouse Without Borders. ShareCursor adds the
> missing Mac-to-Windows path, while Mouse Without Borders serves Windows-only
> desks through PowerToys.

Microsoft Mouse Without Borders is a free PowerToys utility for controlling up
to four Windows computers. It is **Windows-only**. ShareCursor covers the same
desk workflow when one computer is a **Mac**, with encrypted clipboard and file
sharing over the local network.

Compared against the [official Microsoft PowerToys documentation](https://learn.microsoft.com/windows/powertoys/mouse-without-borders)
on **August 22, 2026**.

| Feature | ShareCursor | Mouse Without Borders |
| --- | --- | --- |
| Price | Free & open source | Free (Microsoft PowerToys) |
| Works on Mac | **Yes** | No (Windows only) |
| Works on Windows | Yes | Yes |
| Open source | Yes (MIT / Apache) | Yes (PowerToys) |
| Clipboard (text + images) | Yes | Yes |
| File transfer | Yes | Yes |
| Pairing security | X25519 + ChaCha20 encryption | Security key + computer name |
| Auto discovery | **mDNS (no IPs)** | Security key + machine names |
| Input transport | **UDP, ~6 µs** | Network transport |

## The platform decision

Mouse Without Borders is a direct fit when every computer runs Windows 10 or
Windows 11. It is integrated into PowerToys, supports up to four computers and
includes clipboard and file transfer. There is no macOS client, so it cannot
bridge a Mac and PC.

ShareCursor currently supports macOS and Windows and is designed around a
two-platform desk. It is an independent pre-release project rather than a
Microsoft utility. That gives users an inspectable open protocol and
cross-platform support, while Mouse Without Borders offers the maturity and
distribution of PowerToys.

## Clipboard and files

Both tools move more than input. Microsoft documents clipboard sharing and file
transfer between connected Windows computers. ShareCursor synchronizes clipboard
text and images between Mac and Windows and sends files through a separate
reliable encrypted channel.

## Setup differences

Mouse Without Borders pairs Windows machines using a security key and computer
name inside PowerToys. ShareCursor advertises devices over mDNS, authenticates the
pair and exchanges the screen arrangement. Both require local firewall access
and work best when the computers can reach each other directly on the same LAN.

## Which should you pick?

- **Choose ShareCursor** if either machine is a **Mac**. Mouse Without Borders has
  no macOS client.
- **Choose Mouse Without Borders** if **all** machines run Windows and you want
  PowerToys integration and support for up to four PCs.

Mouse Without Borders is the straightforward free choice for Windows-only desks.
ShareCursor brings the same one-keyboard-and-mouse idea to Mac and Windows, with a
dedicated UDP input path, encryption on every channel and mDNS discovery.
ShareCursor's ~6 µs measurement is loopback transport overhead, not a direct
end-to-end benchmark against the Microsoft tool.

- [Download ShareCursor (free)](https://github.com/phun333/ShareCursor/releases)
- [Mac and Windows clipboard guide](https://sharecursor.com/share-clipboard-between-mac-and-windows.md)
- [Setup guide](https://sharecursor.com/how-to-share-mouse-keyboard-mac-windows.md)
- [Keyboard-and-mouse sharing alternatives](https://sharecursor.com/synergy-alternatives.md)
