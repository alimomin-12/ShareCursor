# How to share a mouse & keyboard between Mac and Windows

> Step-by-step guide to share one mouse, keyboard, clipboard and files between a
> Mac and a Windows PC for free with ShareCursor, an open-source software KVM.

You can control both your Mac and your Windows PC with a single keyboard and
mouse — for free, no KVM hardware, no cloud. Here's how with **ShareCursor**, an
open-source software KVM. It takes about three minutes.

## 1. Install on both machines

Download **ShareCursor** from the [releases page](https://github.com/phun333/ShareCursor/releases):
the **.dmg** on your Mac and the **.exe** on your Windows PC. Both launch to the
menu bar (macOS) / system tray (Windows). First launch blocked? See the
[install help](https://github.com/phun333/ShareCursor/blob/main/docs/INSTALL.md).

## 2. Set one shared passphrase & layout

Open **Settings & Monitor Manager** on both machines. Enter the **same
passphrase** (it authenticates and encrypts the connection), then say which
machine sits on which screen edge — e.g. the PC is to the right of the Mac.

## 3. Grant permissions

- **macOS:** System Settings → Privacy & Security → enable **Accessibility** and **Input Monitoring** for ShareCursor.
- **Windows:** allow ShareCursor through the firewall when prompted.

Both machines must be on the same Wi-Fi / LAN.

## 4. Just move your mouse

Slide the cursor into the shared screen edge — your keyboard and mouse now drive
the other computer. Push back to return. Copy on one machine and paste on the
other; the **clipboard syncs automatically**, and you can send files too.

## What ShareCursor shares

| Capability | Behavior |
|---|---|
| Mouse and keyboard | Control crosses when the pointer reaches the configured screen edge |
| Clipboard text | Copy and paste in both directions |
| Clipboard images | Synchronized between macOS and Windows |
| Files | Transferred over the encrypted reliable channel |
| Video/display | Not shared; each computer continues using its own monitor |

If clipboard sharing is your main requirement, see the dedicated guide to
[sharing a clipboard between Mac and Windows](https://sharecursor.com/share-clipboard-between-mac-and-windows.md).

## Common setup problems

### The computers cannot find each other

Confirm that both computers are on the same local network. Guest Wi-Fi often
blocks devices from reaching each other. On Windows, check that the firewall
rule allows ShareCursor. Wired Ethernet also works and can make network
troubleshooting simpler.

### The Mac does not accept remote input

Reopen System Settings, check Accessibility and Input Monitoring, then restart
ShareCursor. macOS permissions are attached to the installed application, so
moving or replacing the app can require approval again.

### The cursor crosses on the wrong side

Open the monitor arrangement and place the Mac and Windows display in the same
physical order as the desk. The shared edge in the layout determines where the
pointer leaves and where it appears.

### Clipboard or files do not arrive

First confirm that mouse control is connected. Test clipboard sync with a short
line of plain text before trying an image or application-specific format.
Clipboard changes made while disconnected are not queued for later delivery.

## Software KVM or hardware KVM?

ShareCursor is a [software KVM](https://sharecursor.com/what-is-a-software-kvm.md):
it shares input over the LAN while every computer keeps its own display. A
hardware KVM physically switches a keyboard, mouse and monitor between
computers. Use hardware when one monitor must display several machines or when
you need BIOS-level control. Use software when both screens stay visible and you
want seamless pointer, clipboard and file movement.

That's it. One keyboard, one mouse, one clipboard across your Mac and Windows PC —
**encrypted, low-latency, and free**.

- [Download ShareCursor (free)](https://github.com/phun333/ShareCursor/releases)
- [Synergy alternatives compared](https://sharecursor.com/synergy-alternatives.md)
- [Back to home](https://sharecursor.com/index.md)
