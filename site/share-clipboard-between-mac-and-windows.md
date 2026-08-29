# How to share clipboard between Mac and Windows

To share a clipboard between a **Mac and Windows PC**, install ShareCursor on both
computers and pair them on the same local network. Copy text or an image on
either machine and paste it on the other. The transfer stays on your LAN and is
encrypted end to end.

- **Price:** Free and open source (MIT / Apache-2.0)
- **Platforms:** macOS and Windows
- **Internet required:** No; the computers communicate over the local network
- **Supported clipboard content:** Text and images
- **Files:** Sent through a separate encrypted file-transfer channel
- **Download:** https://github.com/phun333/ShareCursor/releases

## The quickest method

macOS Universal Clipboard does not support Windows, and Windows clipboard sync
is designed around Windows devices. A cross-platform app fills that gap.
ShareCursor combines clipboard sync with mouse and keyboard sharing, so the two
computers behave like one desk rather than two separate systems.

1. Download ShareCursor for macOS and Windows from the official releases page.
2. Install and start it on both computers connected to the same Wi-Fi or Ethernet LAN.
3. Pair the computers and confirm the same encrypted connection.
4. Copy text or an image normally, switch to the other computer, and paste.

No browser tab, cloud drive or message to yourself is required. Clipboard
monitoring runs in the background while ShareCursor is connected.

## What can be copied?

| Content | How ShareCursor handles it |
|---|---|
| Plain text | Automatic clipboard sync in both directions |
| Images | Raw image clipboard data is synchronized |
| Files | Sent through the separate encrypted file-transfer channel |
| Passwords and sensitive text | Transferred if copied while connected; treat the paired computer as trusted |

Clipboard formats differ between operating systems, so rich
application-specific formatting may not always survive a cross-platform paste.
Plain text and images are the most portable formats. Files use ShareCursor's
file-transfer feature instead of pretending to be clipboard text.

## How clipboard security works

Clipboard contents can include passwords, tokens and private notes, so transport
security matters. ShareCursor keeps traffic on the local network. Pairing uses an
authenticated shared secret, while X25519 key exchange and
ChaCha20-Poly1305 protect both the clipboard and file channels. There is no
ShareCursor account and clipboard data is not stored in a ShareCursor cloud
service.

Only pair computers you control. Clear the clipboard after copying a sensitive
value, and disconnect ShareCursor on networks or machines you do not trust.

## Troubleshooting clipboard sync

### Both computers must be connected

Confirm that ShareCursor reports a live connection on both machines. Clipboard
changes made while disconnected cannot be delivered retroactively.

### Check local network and firewall access

Both machines need to reach each other on the same LAN. On Windows, allow
ShareCursor through the firewall. Guest Wi-Fi networks often isolate devices and
prevent local discovery.

### Grant macOS permissions

Open macOS System Settings and confirm that ShareCursor has the permissions
requested during setup. Restart the app after changing a permission.

### Try plain text first

Copy a short line from a basic text editor. If that works but content from one
specific app does not, the source app may use a private clipboard format.
Pasting as plain text is the most reliable cross-platform test.

## Clipboard sharing vs cloud sync

Emailing text to yourself or placing a file in cloud storage can work, but those
methods add an account, internet dependency and extra steps. A LAN clipboard
tool is intended for two computers on the same desk: copy, cross the screen edge,
and paste immediately. Cloud storage remains useful when the machines are not on
the same network or when you need long-term file history.

## Frequently asked questions

**Can I share a clipboard between Mac and Windows?**
Yes. Install a cross-platform clipboard tool such as ShareCursor on both
computers. It can copy text and images between macOS and Windows over the local
network.

**Does ShareCursor use the cloud for clipboard sync?**
No. ShareCursor sends clipboard data directly between the paired computers over
the local network. The connection is authenticated and end-to-end encrypted.

**Can I copy files between Mac and Windows too?**
Yes. ShareCursor includes file transfer over its encrypted reliable channel.
Clipboard sync handles text and images, while file transfer handles files.

**Does Apple Universal Clipboard work with Windows?**
No. Apple Universal Clipboard works between supported Apple devices signed into
the same Apple Account. A separate cross-platform tool is needed for
Mac-to-Windows clipboard sharing.

## Related guides

- [Share one mouse and keyboard between Mac and Windows](https://sharecursor.com/how-to-share-mouse-keyboard-mac-windows.md)
- [What is a software KVM?](https://sharecursor.com/what-is-a-software-kvm.md)
- [Synergy alternatives compared](https://sharecursor.com/synergy-alternatives.md)
